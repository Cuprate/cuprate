use bytemuck::TransparentWrapper;
use monero_oxide::transaction::Transaction;

use cuprate_database::{DatabaseRo, DatabaseRw, DbResult, RuntimeError, StorableVec, WriteMode};
use cuprate_types::VerifiedTransactionInformation;

use crate::ops::tx::get_tx;
use crate::{
    ops::macros::{doc_add_alt_block_inner_invariant, doc_error},
    tables::{Tables, TablesMut},
    types::{AltTransactionInfo, TxHash},
};

/// Adds a [`VerifiedTransactionInformation`] from an alt-block
/// if it is not already in the DB.
///
/// If the transaction is in the main-chain this function will still fill in the
/// [`AltTransactionInfos`](crate::tables::AltTransactionInfos) table, as that
/// table holds data which we don't keep around for main-chain txs.
///
#[doc = doc_add_alt_block_inner_invariant!()]
#[doc = doc_error!()]
pub fn add_alt_transaction_blob(
    tx: &VerifiedTransactionInformation,
    tables: &mut impl TablesMut,
) -> DbResult<()> {
    tables.alt_transaction_infos_mut().put(
        &tx.tx_hash,
        &AltTransactionInfo {
            tx_weight: tx.tx_weight,
            fee: tx.fee,
            tx_hash: tx.tx_hash,
        },
        WriteMode::Normal,
    )?;

    if tables.tx_ids().get(&tx.tx_hash).is_ok()
        || tables.alt_transaction_blobs().get(&tx.tx_hash).is_ok()
    {
        return Ok(());
    }

    tables.alt_transaction_blobs_mut().put(
        &tx.tx_hash,
        StorableVec::wrap_ref(&[tx.tx_pruned.as_slice(), tx.tx_prunable_blob.as_slice()].concat()),
        WriteMode::Normal,
    )?;

    Ok(())
}

/// Retrieve a [`VerifiedTransactionInformation`] from the database.
///
#[doc = doc_error!()]
pub fn get_alt_transaction(
    tx_hash: &TxHash,
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
) -> DbResult<VerifiedTransactionInformation> {
    let tx_info = tables.alt_transaction_infos().get(tx_hash)?;

    let tx = match tables.alt_transaction_blobs().get(tx_hash) {
        Ok(blob) => Transaction::read(&mut blob.0.as_slice()).unwrap(),
        Err(RuntimeError::KeyNotFound) => get_tx(tx_hash, tapes, tables.tx_ids())?,
        Err(e) => return Err(e),
    };

    let (pruned_tx, tx_prunable_blob) = tx.pruned_with_prunable();

    Ok(VerifiedTransactionInformation {
        tx_pruned: pruned_tx.serialize(),
        tx_prunable_blob,
        tx: pruned_tx,
        tx_weight: tx_info.tx_weight,
        fee: tx_info.fee,
        tx_hash: tx_info.tx_hash,
    })
}
