use crate::tables::{Tables, TablesMut};
use crate::types::{AltTransactionInfo, TxHash};
use bytemuck::TransparentWrapper;
use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError, StorableVec};
use cuprate_types::VerifiedTransactionInformation;
use monero_serai::transaction::Transaction;

pub fn add_alt_transaction_blob(
    tx: &VerifiedTransactionInformation,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    tables.alt_transaction_infos_mut().put(
        &tx.tx_hash,
        &AltTransactionInfo {
            tx_weight: tx.tx_weight,
            fee: tx.fee,
            tx_hash: tx.tx_hash,
        },
    )?;

    if tables.tx_ids().get(&tx.tx_hash).is_ok()
        || tables.alt_transaction_blobs().get(&tx.tx_hash).is_ok()
    {
        return Ok(());
    }

    tables
        .alt_transaction_blobs_mut()
        .put(&tx.tx_hash, StorableVec::wrap_ref(&tx.tx_blob))
}

pub fn get_alt_transaction(
    tx_hash: &TxHash,
    tables: &impl Tables,
) -> Result<VerifiedTransactionInformation, RuntimeError> {
    let tx_info = tables.alt_transaction_infos().get(tx_hash)?;

    let tx_blob = match tables.alt_transaction_blobs().get(tx_hash) {
        Ok(blob) => blob.0,
        Err(RuntimeError::KeyNotFound) => {
            let tx_id = tables.tx_ids().get(tx_hash)?;

            let blob = tables.tx_blobs().get(&tx_id)?;

            blob.0
        }
        Err(e) => return Err(e),
    };

    Ok(VerifiedTransactionInformation {
        tx: Transaction::read(&mut tx_blob.as_slice()).unwrap(),
        tx_blob,
        tx_weight: tx_info.tx_weight,
        fee: tx_info.fee,
        tx_hash: tx_info.tx_hash,
    })
}
