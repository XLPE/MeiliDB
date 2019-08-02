mod sum_of_typos;
mod number_of_words;
mod words_proximity;
mod sum_of_words_attribute;
mod sum_of_words_position;
mod exact;
mod document_id;

use std::cmp::Ordering;
use crate::RawDocument;

pub use self::{
    sum_of_typos::SumOfTypos,
    number_of_words::NumberOfWords,
    words_proximity::WordsProximity,
    sum_of_words_attribute::SumOfWordsAttribute,
    sum_of_words_position::SumOfWordsPosition,
    exact::Exact,
    document_id::DocumentId,
};

pub trait Criterion: Send + Sync {
    fn prepare(&self, document: &mut RawDocument);

    fn evaluate(&self, lhs: &RawDocument, rhs: &RawDocument) -> Ordering;

    fn name(&self) -> &'static str;

    #[inline]
    fn eq(&self, lhs: &RawDocument, rhs: &RawDocument) -> bool {
        self.evaluate(lhs, rhs) == Ordering::Equal
    }
}

impl<'a, T: Criterion + ?Sized + Send + Sync> Criterion for &'a T {
    fn prepare(&self, document: &mut RawDocument) {
        (**self).prepare(document)
    }

    fn evaluate(&self, lhs: &RawDocument, rhs: &RawDocument) -> Ordering {
        (**self).evaluate(lhs, rhs)
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn eq(&self, lhs: &RawDocument, rhs: &RawDocument) -> bool {
        (**self).eq(lhs, rhs)
    }
}

impl<T: Criterion + ?Sized> Criterion for Box<T> {
    fn prepare(&self, document: &mut RawDocument) {
        (**self).prepare(document)
    }

    fn evaluate(&self, lhs: &RawDocument, rhs: &RawDocument) -> Ordering {
        (**self).evaluate(lhs, rhs)
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn eq(&self, lhs: &RawDocument, rhs: &RawDocument) -> bool {
        (**self).eq(lhs, rhs)
    }
}

#[derive(Default)]
pub struct CriteriaBuilder<'a> {
    inner: Vec<Box<dyn Criterion + 'a>>
}

impl<'a> CriteriaBuilder<'a>
{
    pub fn new() -> CriteriaBuilder<'a> {
        CriteriaBuilder { inner: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> CriteriaBuilder<'a> {
        CriteriaBuilder { inner: Vec::with_capacity(capacity) }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }

    pub fn add<C: 'a>(mut self, criterion: C) -> CriteriaBuilder<'a>
    where C: Criterion,
    {
        self.push(criterion);
        self
    }

    pub fn push<C: 'a>(&mut self, criterion: C)
    where C: Criterion,
    {
        self.inner.push(Box::new(criterion));
    }

    pub fn build(self) -> Criteria<'a> {
        Criteria { inner: self.inner }
    }
}

pub struct Criteria<'a> {
    inner: Vec<Box<dyn Criterion + 'a>>,
}

impl<'a> Default for Criteria<'a> {
    fn default() -> Self {
        CriteriaBuilder::with_capacity(7)
            .add(SumOfTypos)
            .add(NumberOfWords)
            .add(WordsProximity)
            .add(SumOfWordsAttribute)
            .add(SumOfWordsPosition)
            .add(Exact)
            .add(DocumentId)
            .build()
    }
}

impl<'a> AsRef<[Box<dyn Criterion + 'a>]> for Criteria<'a> {
    fn as_ref(&self) -> &[Box<dyn Criterion + 'a>] {
        &self.inner
    }
}
