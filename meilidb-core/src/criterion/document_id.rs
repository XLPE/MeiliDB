use std::cmp::Ordering;
use crate::criterion::Criterion;
use crate::RawDocument;

#[derive(Debug, Clone, Copy)]
pub struct DocumentId;

impl Criterion for DocumentId {
    fn prepare(&self, _document: &mut RawDocument) {
        // nothing to prepare...
    }

    fn evaluate(&self, lhs: &RawDocument, rhs: &RawDocument) -> Ordering {
        lhs.id.cmp(&rhs.id)
    }

    fn name(&self) -> &'static str {
        "DocumentId"
    }
}
