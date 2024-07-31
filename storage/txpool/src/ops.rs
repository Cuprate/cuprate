use crate::ops::key_images::add_tx_key_images;
use crate::tables::TablesMut;
use crate::types::{TransactionHash, TransactionInfo};
use bytemuck::TransparentWrapper;
use cuprate_database::{RuntimeError, StorableVec};
use cuprate_types::TransactionVerificationData;
use monero_serai::transaction::{Input, Transaction};
use std::sync::{Arc, Mutex};

mod add_transaction;
mod key_images;

fn add_transaction(
    tx: Arc<TransactionVerificationData>,
    state_stem: bool,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
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
    if let Some(_) = add_tx_key_images(&tx.tx.prefix.inputs, &tx.tx_hash, kis_table)? {
        todo!("Handle double spent tx");
    }

    Ok(())
}

fn ki_from_input(input: &Input) -> [u8; 32] {
    match input {
        Input::ToKey { key_image, .. } => key_image.compress().0,
        Input::Gen(_) => panic!("miner tx cannot be added to the txpool"),
    }
}

fn take_transaction(
    tx_hash: &TransactionHash,
    tables: &mut impl TablesMut,
) -> Result<TransactionVerificationData, RuntimeError> {
    let tx_blob = tables.transaction_blobs_mut().take(tx_hash)?.0;

    let tx_info = tables.transaction_infomation_mut().take(tx_hash)?;

    let cached_verification_state = tables.cached_verification_state_mut().take(tx_hash)?.into();

    let tx = Transaction::read(&mut tx_blob.as_slice())?;

    let kis_table = tables.spent_key_images_mut();
    for ki in tx.prefix.inputs.iter().map(ki_from_input) {
        kis_table.delete(&ki)?;
    }

    Ok(TransactionVerificationData {
        tx,
        tx_blob,
        tx_weight: tx_info.weight,
        fee: tx_info.fee,
        tx_hash: *tx_hash,
        cached_verification_state: Mutex::new(cached_verification_state),
    })
}
