use sdset::SetBuf;
use std::collections::HashSet;
use std::sync::Arc;

use arc_swap::{ArcSwap, Lease};
use meilidb_core::criterion::Criteria;
use meilidb_core::{DocIndex, Store, DocumentId, QueryBuilder};
use meilidb_schema::Schema;
use rmp_serde::decode::Error as RmpError;
use serde::de;

use crate::ranked_map::RankedMap;
use crate::serde::Deserializer;

use super::{Error, CustomSettings};
use super::{
    RawIndex,
    DocumentsAddition, DocumentsDeletion,
    SynonymsAddition, SynonymsDeletion,
};

#[derive(Copy, Clone)]
pub struct IndexStats {
    pub number_of_words: usize,
    pub number_of_documents: usize,
    pub number_attrs_in_ranked_map: usize,
}

#[derive(Clone)]
pub struct Index(pub ArcSwap<InnerIndex>);

pub struct InnerIndex {
    pub words: fst::Set,
    pub synonyms: fst::Set,
    pub schema: Schema,
    pub ranked_map: RankedMap,
    pub raw: RawIndex, // TODO this will be a snapshot in the future
}

impl Index {
    pub fn from_raw(raw: RawIndex) -> Result<Index, Error> {
        let words = match raw.main.words_set()? {
            Some(words) => words,
            None => fst::Set::default(),
        };

        let synonyms = match raw.main.synonyms_set()? {
            Some(synonyms) => synonyms,
            None => fst::Set::default(),
        };

        let schema = match raw.main.schema()? {
            Some(schema) => schema,
            None => return Err(Error::SchemaMissing),
        };

        let ranked_map = match raw.main.ranked_map()? {
            Some(map) => map,
            None => RankedMap::default(),
        };

        let inner = InnerIndex { words, synonyms, schema, ranked_map, raw };
        let index = Index(ArcSwap::new(Arc::new(inner)));

        Ok(index)
    }

    pub fn stats(&self) -> Result<IndexStats, rocksdb::Error> {
        let lease = self.0.lease();

        Ok(IndexStats {
            number_of_words: lease.words.len(),
            number_of_documents: lease.raw.documents.len()?,
            number_attrs_in_ranked_map: lease.ranked_map.len(),
        })
    }

    pub fn query_builder(&self) -> QueryBuilder<IndexLoad> {
        let index = IndexLoad(self.0.load());
        QueryBuilder::new(index)
    }

    pub fn query_builder_with_criteria<'c>(
        &self,
        criteria: Criteria<'c>,
    ) -> QueryBuilder<'c, IndexLoad>
    {
        let index = IndexLoad(self.0.load());
        QueryBuilder::with_criteria(index, criteria)
    }

    pub fn lease_inner(&self) -> Lease<Arc<InnerIndex>> {
        self.0.lease()
    }

    pub fn schema(&self) -> Schema {
        self.0.lease().schema.clone()
    }

    pub fn custom_settings(&self) -> CustomSettings {
        self.0.lease().raw.custom.clone()
    }

    pub fn documents_addition(&self) -> DocumentsAddition {
        let ranked_map = self.0.lease().ranked_map.clone();
        DocumentsAddition::new(self, ranked_map)
    }

    pub fn documents_deletion(&self) -> DocumentsDeletion {
        let ranked_map = self.0.lease().ranked_map.clone();
        DocumentsDeletion::new(self, ranked_map)
    }

    pub fn synonyms_addition(&self) -> SynonymsAddition {
        SynonymsAddition::new(self)
    }

    pub fn synonyms_deletion(&self) -> SynonymsDeletion {
        SynonymsDeletion::new(self)
    }

    pub fn document<T>(
        &self,
        fields: Option<&HashSet<&str>>,
        id: DocumentId,
    ) -> Result<Option<T>, RmpError>
    where T: de::DeserializeOwned,
    {
        let schema = &self.lease_inner().schema;
        let fields = fields
            .map(|fields| {
                fields
                    .iter()
                    .filter_map(|name| schema.attribute(name))
                    .collect()
            });

        let mut deserializer = Deserializer {
            document_id: id,
            index: &self,
            fields: fields.as_ref(),
        };

        // TODO: currently we return an error if all document fields are missing,
        //       returning None would have been better
        T::deserialize(&mut deserializer).map(Some)
    }
}

#[derive(Clone)]
pub struct IndexLoad(Arc<InnerIndex>);

impl Store for IndexLoad {
    type Error = Error;

    fn words(&self) -> Result<&fst::Set, Self::Error> {
        Ok(&self.0.words)
    }

    fn word_indexes(&self, word: &[u8]) -> Result<Option<SetBuf<DocIndex>>, Self::Error> {
        Ok(self.0.raw.words.doc_indexes(word)?)
    }

    fn synonyms(&self) -> Result<&fst::Set, Self::Error> {
        Ok(&self.0.synonyms)
    }

    fn alternatives_to(&self, word: &[u8]) -> Result<Option<fst::Set>, Self::Error> {
        Ok(self.0.raw.synonyms.alternatives_to(word)?)
    }
}
