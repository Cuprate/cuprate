use crate::tables::SpentKeyImages;
use crate::types::TransactionHash;
use cuprate_database::{DatabaseRw, RuntimeError};
use monero_serai::transaction::Input;

pub fn add_tx_key_images(
    inputs: &[Input],
    tx_hash: &TransactionHash,
    kis_table: &mut impl DatabaseRw<SpentKeyImages>,
) -> Result<Option<TransactionHash>, RuntimeError> {
    for ki in inputs.iter().map(ki_from_input) {
        if let Ok(double_spend_tx_hash) = kis_table.get(&ki) {
            return Ok(Some(double_spend_tx_hash));
        }

        kis_table.put(&ki, tx_hash)?;
    }

    Ok(None)
}

pub fn remove_tx_key_images(
    inputs: &[Input],
    kis_table: &mut impl DatabaseRw<SpentKeyImages>,
) -> Result<(), RuntimeError> {
    for ki in inputs.iter().map(ki_from_input) {
        kis_table.delete(&ki)?;
    }

    Ok(())
}

fn ki_from_input(input: &Input) -> [u8; 32] {
    match input {
        Input::ToKey { key_image, .. } => key_image.compress().0,
        Input::Gen(_) => panic!("miner tx cannot be added to the txpool"),
    }
}
