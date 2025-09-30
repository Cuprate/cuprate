use bytemuck::TransparentWrapper;
use monero_oxide::transaction::Transaction;

use cuprate_database::{DatabaseRo, DatabaseRw, DbResult, RuntimeError, StorableVec};
use cuprate_types::VerifiedTransactionInformation;

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
    )?;

    if tables.tx_ids().get(&tx.tx_hash).is_ok()
        || tables.alt_transaction_blobs().get(&tx.tx_hash).is_ok()
    {
        return Ok(());
    }

    tables
        .alt_transaction_blobs_mut()
        .put(&tx.tx_hash, StorableVec::wrap_ref(&tx.tx_blob))?;

    Ok(())
}

/// Retrieve a [`VerifiedTransactionInformation`] from the database.
///
#[doc = doc_error!()]
pub fn get_alt_transaction(
    tx_hash: &TxHash,
    tables: &impl Tables,
) -> DbResult<VerifiedTransactionInformation> {
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
