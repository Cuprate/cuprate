use monero_serai::transaction::Input;

use cuprate_database::{DatabaseRw, RuntimeError};

use crate::{tables::SpentKeyImages, types::TransactionHash, TxPoolWriteError};

/// Adds the transaction key images to the [`SpentKeyImages`] table.
///
/// This function will return an error if any of the key images are already spent.
///
/// # Panics
/// This function will panic if any of the [`Input`]s are not [`Input::ToKey`]
pub fn add_tx_key_images(
    inputs: &[Input],
    tx_hash: &TransactionHash,
    kis_table: &mut impl DatabaseRw<SpentKeyImages>,
) -> Result<(), TxPoolWriteError> {
    for ki in inputs.iter().map(ki_from_input) {
        if let Ok(double_spend_tx_hash) = kis_table.get(&ki) {
            return Err(TxPoolWriteError::DoubleSpend(double_spend_tx_hash));
        }

        kis_table.put(&ki, tx_hash)?;
    }

    Ok(())
}

/// Removes key images from the [`SpentKeyImages`] table.
///
/// # Panics
/// This function will panic if any of the [`Input`]s are not [`Input::ToKey`]
pub fn remove_tx_key_images(
    inputs: &[Input],
    kis_table: &mut impl DatabaseRw<SpentKeyImages>,
) -> Result<(), RuntimeError> {
    for ki in inputs.iter().map(ki_from_input) {
        kis_table.delete(&ki)?;
    }

    Ok(())
}

/// Maps an input to a key image.
///
/// # Panics
/// This function will panic if the [`Input`] is not [`Input::ToKey`]
fn ki_from_input(input: &Input) -> [u8; 32] {
    match input {
        Input::ToKey { key_image, .. } => key_image.compress().0,
        Input::Gen(_) => panic!("miner tx cannot be added to the txpool"),
    }
}
