use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;

use deunicode::deunicode_with_tofu;
use meilidb_core::{DocumentId, DocIndex};
use meilidb_schema::SchemaAttr;
use meilidb_tokenizer::{is_cjk, Tokenizer, SeqTokenizer, Token};
use sdset::SetBuf;

type Word = Vec<u8>; // TODO make it be a SmallVec

pub struct Indexer {
    word_limit: usize, // the maximum number of indexed words
    words_doc_indexes: BTreeMap<Word, Vec<DocIndex>>,
    docs_words: HashMap<DocumentId, Vec<Word>>,
}

#[derive(Debug)]
pub struct Indexed {
    pub words_doc_indexes: BTreeMap<Word, SetBuf<DocIndex>>,
    pub docs_words: HashMap<DocumentId, fst::Set>,
}

impl Indexer {
    pub fn new() -> Indexer {
        Indexer::with_word_limit(1000)
    }

    pub fn with_word_limit(limit: usize) -> Indexer {
        Indexer {
            word_limit: limit,
            words_doc_indexes: BTreeMap::new(),
            docs_words: HashMap::new(),
        }
    }

    pub fn index_text(&mut self, id: DocumentId, attr: SchemaAttr, text: &str) {
        for token in Tokenizer::new(text) {
            let must_continue = index_token(
                token,
                id,
                attr,
                self.word_limit,
                &mut self.words_doc_indexes,
                &mut self.docs_words,
            );

            if !must_continue { break }
        }
    }

    pub fn index_text_seq<'a, I>(&mut self, id: DocumentId, attr: SchemaAttr, iter: I)
    where I: IntoIterator<Item=&'a str>,
    {
        let iter = iter.into_iter();
        for token in SeqTokenizer::new(iter) {
            let must_continue = index_token(
                token,
                id,
                attr,
                self.word_limit,
                &mut self.words_doc_indexes,
                &mut self.docs_words,
            );

            if !must_continue { break }
        }
    }

    pub fn build(self) -> Indexed {
        let words_doc_indexes = self.words_doc_indexes
            .into_iter()
            .map(|(word, indexes)| (word, SetBuf::from_dirty(indexes)))
            .collect();

        let docs_words = self.docs_words
            .into_iter()
            .map(|(id, mut words)| {
                words.sort_unstable();
                words.dedup();
                (id, fst::Set::from_iter(words).unwrap())
            })
            .collect();

        Indexed { words_doc_indexes, docs_words }
    }
}

fn index_token(
    token: Token,
    id: DocumentId,
    attr: SchemaAttr,
    word_limit: usize,
    words_doc_indexes: &mut BTreeMap<Word, Vec<DocIndex>>,
    docs_words: &mut HashMap<DocumentId, Vec<Word>>,
) -> bool
{
    if token.word_index >= word_limit { return false }

    let lower = token.word.to_lowercase();
    let token = Token { word: &lower, ..token };
    match token_to_docindex(id, attr, token) {
        Some(docindex) => {
            let word = Vec::from(token.word);
            words_doc_indexes.entry(word.clone()).or_insert_with(Vec::new).push(docindex);
            docs_words.entry(id).or_insert_with(Vec::new).push(word);
        },
        None => return false,
    }

    if !lower.contains(is_cjk) {
        let unidecoded = deunicode_with_tofu(&lower, "");
        if unidecoded != lower {
            let token = Token { word: &unidecoded, ..token };
            match token_to_docindex(id, attr, token) {
                Some(docindex) => {
                    let word = Vec::from(token.word);
                    words_doc_indexes.entry(word.clone()).or_insert_with(Vec::new).push(docindex);
                    docs_words.entry(id).or_insert_with(Vec::new).push(word);
                },
                None => return false,
            }
        }
    }

    true
}

fn token_to_docindex(id: DocumentId, attr: SchemaAttr, token: Token) -> Option<DocIndex> {
    let word_index = u16::try_from(token.word_index).ok()?;
    let char_index = u16::try_from(token.char_index).ok()?;
    let char_length = u16::try_from(token.word.chars().count()).ok()?;

    let docindex = DocIndex {
        document_id: id,
        attribute: attr.0,
        word_index,
        char_index,
        char_length,
    };

    Some(docindex)
}
