use crate::ops::key_images::{add_tx_key_images, remove_tx_key_images};
use crate::tables::TablesMut;
use crate::types::{TransactionHash, TransactionInfo};
use crate::TxPoolWriteError;
use bytemuck::TransparentWrapper;
use cuprate_database::{RuntimeError, StorableVec};
use cuprate_types::TransactionVerificationData;
use monero_serai::transaction::{Input, Transaction};
use std::sync::Arc;

/// Adds a transaction to the tx-pool.
///
/// This function fills in all tables necessary to add the transaction to the pool.
///
/// # Panics
/// This function will panic if the transactions inputs are not all of type [`Input::ToKey`].
///
fn add_transaction(
    tx: Arc<TransactionVerificationData>,
    state_stem: bool,
    tables: &mut impl TablesMut,
) -> Result<(), TxPoolWriteError> {
    // Add the tx blob to table 0.
    tables
        .transaction_blobs_mut()
        .put(&tx.tx_hash, StorableVec::wrap_ref(&tx.tx_blob))?;

    // Add the tx info to table 1.
    tables.transaction_infomation_mut().put(
        &tx.tx_hash,
        &TransactionInfo {
            fee: tx.fee,
            weight: tx.tx_weight,
            state_stem,
            double_spend_seen: false,
        },
    )?;

    // Add the cached verification state to table 2.
    let cached_verification_state = tx.cached_verification_state.lock().unwrap().into();
    tables
        .cached_verification_state_mut()
        .put(&tx.tx_hash, &cached_verification_state)?;

    // Add the tx key images to table 3.
    let kis_table = tables.spent_key_images_mut();
    add_tx_key_images(&tx.tx.prefix.inputs, &tx.tx_hash, kis_table)?;

    Ok(())
}

/// Removes a transaction from the transaction pool.
fn remove_transaction(
    tx_hash: &TransactionHash,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    // Remove the tx blob from table 0.
    let tx_blob = tables.transaction_blobs_mut().take(tx_hash)?.0;

    // Remove the tx info from table 1.
    tables.transaction_infomation_mut().delete(tx_hash)?;

    // Remove the cached verification state from table 2.
    tables.cached_verification_state_mut().delete(tx_hash)?;

    // Remove the tx key images from table 3.
    let tx =
        Transaction::read(&mut tx_blob.as_slice()).expect("Tx in the tx-pool must be parseable");
    let kis_table = tables.spent_key_images_mut();
    remove_tx_key_images(&tx.prefix.inputs, kis_table)?;

    Ok(())
}
