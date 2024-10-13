//! Transaction read ops.
//!
//! This module handles reading full transaction data, like getting a transaction from the pool.
use std::sync::Mutex;

use monero_serai::transaction::Transaction;

use cuprate_database::{DatabaseRo, RuntimeError};
use cuprate_types::{TransactionVerificationData, TxVersion};

use crate::tables::TransactionInfos;
use crate::types::TxStateFlags;
use crate::{tables::Tables, types::TransactionHash};

/// Gets the [`TransactionVerificationData`] of a transaction in the tx-pool, leaving the tx in the pool.
pub fn get_transaction_verification_data(
    tx_hash: &TransactionHash,
    tables: &impl Tables,
) -> Result<TransactionVerificationData, RuntimeError> {
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

pub fn in_stem_pool(
    tx_hash: &TransactionHash,
    tx_infos: &impl DatabaseRo<TransactionInfos>,
) -> Result<bool, RuntimeError> {
    Ok(tx_infos
        .get(tx_hash)?
        .flags
        .contains(TxStateFlags::STATE_STEM))
}
