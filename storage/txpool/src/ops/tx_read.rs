use crate::tables::{Tables, TransactionBlobs};
use crate::types::TransactionHash;
use cuprate_database::{DatabaseRw, RuntimeError};
use cuprate_types::TransactionVerificationData;
use monero_serai::transaction::Transaction;
use std::sync::Mutex;

/// Gets the [`TransactionVerificationData`] of a transaction in the tx-pool, leaving the tx in the pool.
pub fn get_transaction_verification_data(
    tx_hash: &TransactionHash,
    tables: &impl Tables,
) -> Result<TransactionVerificationData, RuntimeError> {
    let tx_blob = tables.transaction_blobs_mut().get(tx_hash)?.0;

    let tx_info = tables.transaction_infomation_mut().get(tx_hash)?;

    let cached_verification_state = tables.cached_verification_state_mut().get(tx_hash)?.into();

    Ok(TransactionVerificationData {
        tx: Transaction::read(&mut tx_blob.as_slice())
            .expect("Tx in the tx-pool must be parseable"),
        tx_blob,
        tx_weight: tx_info.weight,
        fee: tx_info.fee,
        tx_hash: *tx_hash,
        cached_verification_state: Mutex::new(cached_verification_state),
    })
}
