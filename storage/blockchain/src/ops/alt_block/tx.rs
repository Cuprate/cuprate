use bytemuck::TransparentWrapper;
use cuprate_types::VerifiedTransactionInformation;
use monero_oxide::transaction::Transaction;
use tapes::MmapFile;
use crate::database::{ALT_TRANSACTION_BLOBS, ALT_TRANSACTION_INFOS, TX_IDS};
use crate::error::{BlockchainError, DbResult};
use crate::ops::tx::get_tx;
use crate::{
    ops::macros::{doc_add_alt_block_inner_invariant, doc_error},
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
    tx_rw: &mut heed::RwTxn,
) -> DbResult<()> {
    ALT_TRANSACTION_INFOS.get().unwrap().put(
        tx_rw,
        &tx.tx_hash,
        &AltTransactionInfo {
            tx_weight: tx.tx_weight,
            fee: tx.fee,
            tx_hash: tx.tx_hash,
        },
    )?;

    if TX_IDS.get().unwrap().get(tx_rw, &tx.tx_hash).is_ok()
        || ALT_TRANSACTION_BLOBS
            .get()
            .unwrap()
            .get(tx_rw, &tx.tx_hash)
            .as_ref()
            .is_ok_and(Option::is_some)
    {
        return Ok(());
    }

    // TODO: the below can be made more efficient pretty easily.
    ALT_TRANSACTION_BLOBS.get().unwrap().put(
        tx_rw,
        &tx.tx_hash,
        [tx.tx_pruned.as_slice(), tx.tx_prunable_blob.as_slice()]
            .concat()
            .as_slice(),
    )?;

    Ok(())
}

/// Retrieve a [`VerifiedTransactionInformation`] from the database.
///
#[doc = doc_error!()]
pub fn get_alt_transaction(
    tx_hash: &TxHash,
    tx_ro: &heed::RoTxn,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<VerifiedTransactionInformation> {
    let tx_info = ALT_TRANSACTION_INFOS
        .get()
        .unwrap()
        .get(tx_ro, tx_hash)?
        .ok_or(BlockchainError::NotFound)?;

    let tx = match ALT_TRANSACTION_BLOBS.get().unwrap().get(tx_ro, tx_hash)? {
        Some(mut tx_blob) => Transaction::read(&mut tx_blob).unwrap(),
        None => get_tx(tx_hash, tx_ro, tapes)?,
    };

    let tx_weight = tx_info.tx_weight;
    let fee = tx_info.fee;
    let (tx, tx_prunable_blob) = tx.pruned_with_prunable();

    Ok(VerifiedTransactionInformation {
        tx_prunable_blob,
        tx_pruned: tx.serialize(),
        tx_weight,
        fee,
        tx_hash: *tx_hash,
        tx,
    })
}
