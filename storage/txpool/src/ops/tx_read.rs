//! Transaction read ops.
//!
//! This module handles reading full transaction data, like getting a transaction from the pool.

use fjall::Readable;
use monero_oxide::transaction::Transaction;

use crate::error::TxPoolError;
use crate::txpool::TxpoolDatabase;
use crate::types::TransactionInfo;
use crate::types::{TransactionHash, TxStateFlags};
use cuprate_types::rpc::PoolTxInfo;
use cuprate_types::{TransactionVerificationData, TxVersion};

/// Gets the [`TransactionVerificationData`] of a transaction in the tx-pool, leaving the tx in the pool.
pub fn get_transaction_verification_data(
    tx_hash: &TransactionHash,
    snapshot: &fjall::Snapshot,
    db: &TxpoolDatabase,
) -> Result<TransactionVerificationData, TxPoolError> {
    let tx_blob = snapshot
        .get(&db.tx_blobs, &tx_hash)?
        .ok_or(TxPoolError::NotFound)?
        .to_vec();

    let tx_info = snapshot
        .get(&db.tx_infos, &tx_hash)?
        .ok_or(TxPoolError::NotFound)?;

    let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

    let tx =
        Transaction::read(&mut tx_blob.as_slice()).expect("Tx in the tx-pool must be parseable");

    Ok(TransactionVerificationData {
        version: TxVersion::from_raw(tx.version()).expect("Tx in tx-pool has invalid version"),
        tx,
        tx_blob,
        tx_weight: tx_info.weight,
        fee: tx_info.fee,
        tx_hash: *tx_hash,
        cached_verification_state: tx_info.cached_verification_state.into(),
    })
}

/// Returns `true` if the transaction with the given hash is in the stem pool.
///
/// # Errors
/// This will return an [`Err`] if the transaction is not in the pool.
pub fn in_stem_pool(
    tx_hash: &TransactionHash,
    snapshot: &fjall::Snapshot,
    db: &TxpoolDatabase,
) -> Result<bool, TxPoolError> {
    let tx_info = snapshot
        .get(&db.tx_infos, tx_hash)?
        .ok_or(TxPoolError::NotFound)?;

    let tx_info: TransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

    Ok(tx_info.flags.contains(TxStateFlags::STATE_STEM))
}
