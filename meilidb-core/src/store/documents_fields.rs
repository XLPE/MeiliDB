use heed::types::{ByteSlice, OwnedType};
use heed::Result as ZResult;
use meilidb_schema::SchemaAttr;

use super::DocumentAttrKey;
use crate::DocumentId;

#[derive(Copy, Clone)]
pub struct DocumentsFields {
    pub(crate) documents_fields: heed::Database<OwnedType<DocumentAttrKey>, ByteSlice>,
}

impl DocumentsFields {
    pub fn put_document_field(
        self,
        writer: &mut heed::RwTxn,
        document_id: DocumentId,
        attribute: SchemaAttr,
        value: &[u8],
    ) -> ZResult<()> {
        let key = DocumentAttrKey::new(document_id, attribute);
        self.documents_fields.put(writer, &key, value)
    }

    pub fn del_all_document_fields(
        self,
        writer: &mut heed::RwTxn,
        document_id: DocumentId,
    ) -> ZResult<usize> {
        let start = DocumentAttrKey::new(document_id, SchemaAttr::min());
        let end = DocumentAttrKey::new(document_id, SchemaAttr::max());
        self.documents_fields.delete_range(writer, &(start..=end))
    }

    pub fn clear(self, writer: &mut heed::RwTxn) -> ZResult<()> {
        self.documents_fields.clear(writer)
    }

    pub fn document_attribute<'txn>(
        self,
        reader: &'txn heed::RoTxn,
        document_id: DocumentId,
        attribute: SchemaAttr,
    ) -> ZResult<Option<&'txn [u8]>> {
        let key = DocumentAttrKey::new(document_id, attribute);
        self.documents_fields.get(reader, &key)
    }

    pub fn document_fields<'txn>(
        self,
        reader: &'txn heed::RoTxn,
        document_id: DocumentId,
    ) -> ZResult<DocumentFieldsIter<'txn>> {
        let start = DocumentAttrKey::new(document_id, SchemaAttr::min());
        let end = DocumentAttrKey::new(document_id, SchemaAttr::max());
        let iter = self.documents_fields.range(reader, &(start..=end))?;
        Ok(DocumentFieldsIter { iter })
    }
}

pub struct DocumentFieldsIter<'txn> {
    iter: heed::RoRange<'txn, OwnedType<DocumentAttrKey>, ByteSlice>,
}

impl<'txn> Iterator for DocumentFieldsIter<'txn> {
    type Item = ZResult<(SchemaAttr, &'txn [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(Ok((key, bytes))) => {
                let attr = SchemaAttr(key.attr.get());
                Some(Ok((attr, bytes)))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}
