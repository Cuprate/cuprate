//! Transaction writing ops.
//!
//! This module handles writing full transaction data, like removing or adding a transaction.
use std::sync::Arc;

use bytemuck::TransparentWrapper;
use monero_serai::transaction::{Input, NotPruned, Transaction};

use cuprate_database::{RuntimeError, StorableVec, DatabaseRw};
use cuprate_types::TransactionVerificationData;

use crate::{
    ops::key_images::{add_tx_key_images, remove_tx_key_images},
    tables::TablesMut,
    types::{TransactionHash, TransactionInfo},
    TxPoolWriteError,
};

/// Adds a transaction to the tx-pool.
///
/// This function fills in all tables necessary to add the transaction to the pool.
///
/// # Panics
/// This function will panic if the transactions inputs are not all of type [`Input::ToKey`].
pub fn add_transaction(
    tx: &TransactionVerificationData,
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
            flags: todo!(),
            _padding:  [0; 7],
        },
    )?;

    // Add the cached verification state to table 2.
    let cached_verification_state = tx.cached_verification_state.lock().unwrap().clone().into();
    tables
        .cached_verification_state_mut()
        .put(&tx.tx_hash, &cached_verification_state)?;

    // Add the tx key images to table 3.
    let kis_table = tables.spent_key_images_mut();
    add_tx_key_images(&tx.tx.prefix().inputs, &tx.tx_hash, kis_table)?;

    Ok(())
}

/// Removes a transaction from the transaction pool.
pub fn remove_transaction(
    tx_hash: &TransactionHash,
    mut tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    // Remove the tx blob from table 0.
    let tx_blob = tables.transaction_blobs_mut().take(tx_hash)?.0;

    // Remove the tx info from table 1.
    tables.transaction_infomation_mut().delete(tx_hash)?;

    // Remove the cached verification state from table 2.
    tables.cached_verification_state_mut().delete(tx_hash)?;

    // Remove the tx key images from table 3.
    let tx =
        Transaction::<NotPruned>::read(&mut tx_blob.as_slice()).expect("Tx in the tx-pool must be parseable");
    let kis_table = tables.spent_key_images_mut();
    remove_tx_key_images(&tx.prefix().inputs, kis_table)?;

    Ok(())
}
