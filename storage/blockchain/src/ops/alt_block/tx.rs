use crate::error::{BlockchainError, DbResult};
use crate::ops::tx::get_tx;
use crate::{
    ops::macros::{doc_add_alt_block_inner_invariant, doc_error},
    types::{AltTransactionInfo, TxHash},
    BlockchainDatabase,
};
use bytemuck::TransparentWrapper;
use cuprate_types::VerifiedTransactionInformation;
use fjall::Readable;
use monero_oxide::transaction::Transaction;

/// TODO.
///
#[doc = doc_add_alt_block_inner_invariant!()]
#[doc = doc_error!()]
pub fn add_alt_transaction_blob(
    db: &BlockchainDatabase,
    tx: &VerifiedTransactionInformation,
    tx_rw: &mut fjall::OwnedWriteBatch,
) -> DbResult<()> {
    tx_rw.insert(
        &db.alt_transaction_infos,
        tx.tx_hash,
        bytemuck::bytes_of(&AltTransactionInfo {
            tx_weight: tx.tx_weight,
            fee: tx.fee,
            tx_hash: tx.tx_hash,
        }),
    );

    if db.tx_ids.contains_key(tx.tx_hash)? || db.alt_transaction_blobs.contains_key(tx.tx_hash)? {
        return Ok(());
    }

    // TODO: the below can be made more efficient pretty easily.
    tx_rw.insert(
        &db.alt_transaction_blobs,
        tx.tx_hash,
        [tx.tx_pruned.as_slice(), tx.tx_prunable_blob.as_slice()]
            .concat()
            .as_slice(),
    );

    Ok(())
}

/// Retrieve a [`VerifiedTransactionInformation`] from the database.
///
#[doc = doc_error!()]
pub fn get_alt_transaction(
    db: &BlockchainDatabase,
    tx_hash: &TxHash,
    tx_ro: &fjall::Snapshot,
    tapes: &impl tapes::TapesRead,
) -> DbResult<VerifiedTransactionInformation> {
    let tx_info = tx_ro
        .get(&db.alt_transaction_infos, tx_hash)?
        .ok_or(BlockchainError::NotFound)?;

    let tx_info: AltTransactionInfo = bytemuck::pod_read_unaligned(tx_info.as_ref());

    let tx = match tx_ro.get(&db.alt_transaction_blobs, tx_hash)? {
        Some(mut tx_blob) => Transaction::read(&mut tx_blob.as_ref()).unwrap(),
        None => get_tx(db, tx_hash, tx_ro, tapes)?,
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
