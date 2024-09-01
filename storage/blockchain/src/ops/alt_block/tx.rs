use crate::tables::{Tables, TablesMut};
use crate::types::{AltTransactionInfo, TxHash};
use bytemuck::TransparentWrapper;
use cuprate_database::{RuntimeError, StorableVec};
use cuprate_types::VerifiedTransactionInformation;

pub fn add_alt_transaction_blob(
    tx_hash: &TxHash,
    tx_block: &StorableVec<u8>,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    if tables.tx_ids().get(&tx_hash).is_ok() || tables.alt_transaction_blobs().get(&tx_hash).is_ok()
    {
        return Ok(());
    }

    tables.alt_transaction_blobs_mut().put(&tx_hash, tx_block)
}

pub fn get_alt_transaction_blob(
    tx_hash: &TxHash,
    tables: &impl Tables,
) -> Result<Vec<u8>, RuntimeError> {
    match tables.alt_transaction_blobs().get(tx_hash) {
        Ok(blob) => Ok(blob.0),
        Err(RuntimeError::KeyNotFound) => {
            let tx_id = tables.tx_ids().get(tx_hash)?;

            let blob = tables.tx_blobs().get(&tx_id)?;

            Ok(blob.0)
        }
        Err(e) => return Err(e),
    }
}
