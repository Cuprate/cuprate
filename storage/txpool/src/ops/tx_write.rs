//! Transaction writing ops.
//!
//! This module handles writing full transaction data, like removing or adding a transaction.
use monero_oxide::transaction::{Pruned, Transaction};

use cuprate_helper::time::current_unix_timestamp;
use cuprate_types::TransactionVerificationData;

use crate::{
    error::TxPoolError,
    free::transaction_blob_hash,
    ops::{
        key_images::{add_tx_key_images, ki_from_input, remove_tx_key_images},
        TxPoolWriteError,
    },
    txpool::TxpoolDatabase,
    types::{KeyImage, TransactionBlobHash, TransactionHash, TransactionInfo, TxStateFlags},
};

/// Adds a transaction to the tx-pool.
///
/// This function fills in all tables necessary to add the transaction to the pool, and returns its blob hash.
///
/// # Panics
/// This function will panic if the transactions inputs are not all of type [`Input::ToKey`](monero_oxide::transaction::Input::ToKey).
pub fn add_transaction(
    tx: &TransactionVerificationData,
    state_stem: bool,
    w: &mut fjall::OwnedWriteBatch,
    db: &TxpoolDatabase,
) -> Result<TransactionBlobHash, TxPoolWriteError> {
    add_tx_key_images(&tx.tx.prefix().inputs, &tx.tx_hash, w, db)?;

    // Add the tx blob.
    w.insert(&db.tx_blobs, tx.tx_hash, tx.tx_blob.as_ref());

    let mut flags = TxStateFlags::empty();
    flags.set(TxStateFlags::STATE_STEM, state_stem);

    // Add the tx info.
    w.insert(
        &db.tx_infos,
        tx.tx_hash,
        bytemuck::bytes_of(&TransactionInfo {
            fee: tx.fee,
            weight: tx.tx_weight,
            received_at: current_unix_timestamp(),
            cached_verification_state: tx.cached_verification_state.into(),
            flags,
            _padding: [0; 6],
        }),
    );

    // Add the blob hash to table 4.
    let blob_hash = transaction_blob_hash(&tx.tx_blob);
    w.insert(&db.known_blob_hashes, blob_hash, tx.tx_hash);

    Ok(blob_hash)
}

/// Removes a transaction from the transaction pool.
///
/// The blob hash is calculated from the tx blob if it is not given.
pub fn remove_transaction(
    tx_hash: &TransactionHash,
    blob_hash: Option<TransactionBlobHash>,
    w: &mut fjall::OwnedWriteBatch,
    db: &TxpoolDatabase,
) -> Result<(), TxPoolError> {
    let tx_blob = db
        .tx_blobs
        .get(tx_hash.as_ref())?
        .ok_or(TxPoolError::NotFound)?;
    let tx = Transaction::<Pruned>::read(&mut tx_blob.as_ref())
        .expect("Tx in the tx-pool must be parseable");

    let key_images = tx
        .prefix()
        .inputs
        .iter()
        .map(ki_from_input)
        .collect::<Vec<_>>();

    let blob_hash = blob_hash.unwrap_or_else(|| transaction_blob_hash(&tx_blob));

    remove_transaction_with_key_images(tx_hash, &blob_hash, &key_images, w, db);

    Ok(())
}

/// Removes a transaction from the transaction pool, given its blob hash and key images.
///
/// Unlike [`remove_transaction`] nothing is read.
///
/// # Invariant
/// `blob_hash` and `key_images` must be the tx's own, this is checked in debug builds.
pub fn remove_transaction_with_key_images(
    tx_hash: &TransactionHash,
    blob_hash: &TransactionBlobHash,
    key_images: &[KeyImage],
    w: &mut fjall::OwnedWriteBatch,
    db: &TxpoolDatabase,
) {
    // Make sure the given blob hash and key images are the tx's own.
    #[cfg(debug_assertions)]
    {
        let tx_blob = db
            .tx_blobs
            .get(tx_hash.as_ref())
            .unwrap()
            .expect("Tx must be in the tx-pool");
        let tx = Transaction::<Pruned>::read(&mut tx_blob.as_ref())
            .expect("Tx in the tx-pool must be parseable");

        assert_eq!(&transaction_blob_hash(&tx_blob), blob_hash);
        assert!(tx
            .prefix()
            .inputs
            .iter()
            .map(ki_from_input)
            .eq(key_images.iter().copied()));
    }

    w.remove(&db.tx_blobs, tx_hash);
    w.remove(&db.tx_infos, tx_hash);

    remove_tx_key_images(key_images, w, db);

    w.remove(&db.known_blob_hashes, blob_hash);
}
