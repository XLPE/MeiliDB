use super::BEU64;
use crate::update::ProcessedUpdateResult;
use heed::types::{OwnedType, SerdeBincode};
use heed::Result as ZResult;

#[derive(Copy, Clone)]
pub struct UpdatesResults {
    pub(crate) updates_results:
        heed::Database<OwnedType<BEU64>, SerdeBincode<ProcessedUpdateResult>>,
}

impl UpdatesResults {
    pub fn last_update_id(
        self,
        reader: &heed::RoTxn,
    ) -> ZResult<Option<(u64, ProcessedUpdateResult)>> {
        match self.updates_results.last(reader)? {
            Some((key, data)) => Ok(Some((key.get(), data))),
            None => Ok(None),
        }
    }

    pub fn put_update_result(
        self,
        writer: &mut heed::RwTxn,
        update_id: u64,
        update_result: &ProcessedUpdateResult,
    ) -> ZResult<()> {
        let update_id = BEU64::new(update_id);
        self.updates_results.put(writer, &update_id, update_result)
    }

    pub fn update_result(
        self,
        reader: &heed::RoTxn,
        update_id: u64,
    ) -> ZResult<Option<ProcessedUpdateResult>> {
        let update_id = BEU64::new(update_id);
        self.updates_results.get(reader, &update_id)
    }
}
