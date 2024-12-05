//! Transaction read ops.
//!
//! This module handles reading full transaction data, like getting a transaction from the pool.
use std::sync::Mutex;

use monero_serai::transaction::Transaction;

use cuprate_database::{DatabaseRo, DbResult};
use cuprate_types::{TransactionVerificationData, TxVersion};

use crate::{
    tables::{Tables, TransactionInfos},
    types::{TransactionHash, TxStateFlags},
};

/// Gets the [`TransactionVerificationData`] of a transaction in the tx-pool, leaving the tx in the pool.
pub fn get_transaction_verification_data(
    tx_hash: &TransactionHash,
    tables: &impl Tables,
) -> DbResult<TransactionVerificationData> {
    let tx_blob = tables.transaction_blobs().get(tx_hash)?.0;

    let tx_info = tables.transaction_infos().get(tx_hash)?;

    let cached_verification_state = tables.cached_verification_state().get(tx_hash)?.into();

    let tx =
        Transaction::read(&mut tx_blob.as_slice()).expect("Tx in the tx-pool must be parseable");

    Ok(TransactionVerificationData {
        version: TxVersion::from_raw(tx.version()).expect("Tx in tx-pool has invalid version"),
        tx,
        tx_blob,
        tx_weight: tx_info.weight,
        fee: tx_info.fee,
        tx_hash: *tx_hash,
        cached_verification_state: Mutex::new(cached_verification_state),
    })
}

/// Returns `true` if the transaction with the given hash is in the stem pool.
///
/// # Errors
/// This will return an [`Err`] if the transaction is not in the pool.
pub fn in_stem_pool(
    tx_hash: &TransactionHash,
    tx_infos: &impl DatabaseRo<TransactionInfos>,
) -> DbResult<bool> {
    Ok(tx_infos
        .get(tx_hash)?
        .flags
        .contains(TxStateFlags::STATE_STEM))
}
