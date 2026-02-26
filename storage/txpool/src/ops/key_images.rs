//! Tx-pool key image ops.
use monero_oxide::transaction::Input;

use crate::error::TxPoolError;
use crate::txpool::TxpoolDatabase;
use crate::{ops::TxPoolWriteError, types::TransactionHash};

/// Adds the transaction key images to the [`SpentKeyImages`] table.
///
/// This function will return an error if any of the key images are already spent.
///
/// # Panics
/// This function will panic if any of the [`Input`]s are not [`Input::ToKey`]
pub(super) fn add_tx_key_images(
    inputs: &[Input],
    tx_hash: &TransactionHash,
    writer: &mut fjall::OwnedWriteBatch,
    db: &TxpoolDatabase,
) -> Result<(), TxPoolWriteError> {
    for ki in inputs.iter().map(ki_from_input) {
        if let Some(ki) = db.spent_key_images.get(ki).map_err(TxPoolError::Fjall)? {
            return Err(TxPoolWriteError::DoubleSpend(
                ki.as_ref().try_into().unwrap(),
            ));
        }

        writer.insert(&db.spent_key_images, ki, tx_hash);
    }

    Ok(())
}

/// Removes key images from the [`SpentKeyImages`] table.
///
/// # Panics
/// This function will panic if any of the [`Input`]s are not [`Input::ToKey`]
pub(super) fn remove_tx_key_images(
    inputs: &[Input],
    writer: &mut fjall::OwnedWriteBatch,
    db: &TxpoolDatabase,
) {
    for ki in inputs.iter().map(ki_from_input) {
        writer.remove(&db.spent_key_images, ki);
    }
}

/// Maps an input to a key image.
///
/// # Panics
/// This function will panic if the [`Input`] is not [`Input::ToKey`]
pub(super) fn ki_from_input(input: &Input) -> [u8; 32] {
    match input {
        Input::ToKey { key_image, .. } => key_image.0,
        Input::Gen(_) => panic!("miner tx cannot be added to the txpool"),
    }
}
