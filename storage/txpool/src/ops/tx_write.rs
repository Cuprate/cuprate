//! Transaction writing ops.
//!
//! This module handles writing full transaction data, like removing or adding a transaction.
use bytemuck::TransparentWrapper;
use monero_oxide::transaction::{NotPruned, Transaction};

use cuprate_database::{DatabaseRw, DbResult, StorableVec};
use cuprate_helper::time::current_unix_timestamp;
use cuprate_types::TransactionVerificationData;

use crate::{
    free::transaction_blob_hash,
    ops::{
        key_images::{add_tx_key_images, remove_tx_key_images},
        TxPoolWriteError,
    },
    tables::TablesMut,
    types::{TransactionHash, TransactionInfo, TxStateFlags},
};

/// Adds a transaction to the tx-pool.
///
/// This function fills in all tables necessary to add the transaction to the pool.
///
/// # Panics
/// This function will panic if the transactions inputs are not all of type [`Input::ToKey`](monero_oxide::transaction::Input::ToKey).
pub fn add_transaction(
    tx: &TransactionVerificationData,
    state_stem: bool,
    tables: &mut impl TablesMut,
) -> Result<(), TxPoolWriteError> {
    // Add the tx blob to table 0.
    tables
        .transaction_blobs_mut()
        .put(&tx.tx_hash, StorableVec::wrap_ref(&tx.tx_blob), false)?;

    let mut flags = TxStateFlags::empty();
    flags.set(TxStateFlags::STATE_STEM, state_stem);

    // Add the tx info to table 1.
    tables.transaction_infos_mut().put(
        &tx.tx_hash,
        &TransactionInfo {
            fee: tx.fee,
            weight: tx.tx_weight,
            received_at: current_unix_timestamp(),
            flags,
            _padding: [0; 7],
        },
        false,
    )?;

    // Add the cached verification state to table 2.
    let cached_verification_state = tx.cached_verification_state.into();
    tables
        .cached_verification_state_mut()
        .put(&tx.tx_hash, &cached_verification_state, false)?;

    // Add the tx key images to table 3.
    let kis_table = tables.spent_key_images_mut();
    add_tx_key_images(&tx.tx.prefix().inputs, &tx.tx_hash, kis_table)?;

    // Add the blob hash to table 4.
    let blob_hash = transaction_blob_hash(&tx.tx_blob);
    tables
        .known_blob_hashes_mut()
        .put(&blob_hash, &tx.tx_hash, false)?;

    Ok(())
}

/// Removes a transaction from the transaction pool.
pub fn remove_transaction(tx_hash: &TransactionHash, tables: &mut impl TablesMut) -> DbResult<()> {
    // Remove the tx blob from table 0.
    let tx_blob = tables.transaction_blobs_mut().take(tx_hash)?.0;

    // Remove the tx info from table 1.
    tables.transaction_infos_mut().delete(tx_hash)?;

    // Remove the cached verification state from table 2.
    tables.cached_verification_state_mut().delete(tx_hash)?;

    // Remove the tx key images from table 3.
    let tx = Transaction::<NotPruned>::read(&mut tx_blob.as_slice())
        .expect("Tx in the tx-pool must be parseable");
    let kis_table = tables.spent_key_images_mut();
    remove_tx_key_images(&tx.prefix().inputs, kis_table)?;

    // Remove the blob hash from table 4.
    let blob_hash = transaction_blob_hash(&tx_blob);
    tables.known_blob_hashes_mut().delete(&blob_hash)?;

    Ok(())
}
