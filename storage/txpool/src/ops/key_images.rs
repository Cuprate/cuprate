//! Tx-pool key image ops.
use std::collections::HashSet;

use monero_oxide::transaction::Input;

use crate::{
    error::TxPoolError,
    ops::TxPoolWriteError,
    txpool::TxpoolDatabase,
    types::{KeyImage, TransactionHash},
};

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
pub(super) fn remove_tx_key_images(
    key_images: &[KeyImage],
    writer: &mut fjall::OwnedWriteBatch,
    db: &TxpoolDatabase,
) {
    for key_image in key_images {
        writer.remove(&db.spent_key_images, key_image);
    }
}

/// Returns the hashes of the txs in the pool that spend any of the key images.
///
/// A tx is returned once for each of the key images it spends. This looks up each key image, or scans
/// the `spent_key_images` keyspace, whichever has fewer key images.
pub fn txs_spending_key_images(
    key_images: &[KeyImage],
    db: &TxpoolDatabase,
) -> Result<Vec<TransactionHash>, TxPoolError> {
    let mut txs = Vec::new();

    // `approximate_len` also counts tombstones, which a scan walks too.
    if key_images.len() <= db.spent_key_images.approximate_len() {
        for key_image in key_images {
            if let Some(tx_hash) = db.spent_key_images.get(key_image)? {
                txs.push(tx_hash.as_ref().try_into().unwrap());
            }
        }
    } else {
        let key_images = key_images.iter().copied().collect::<HashSet<_>>();

        for guard in db.spent_key_images.iter() {
            let (_, tx_hash) =
                guard.into_inner_if(|key_image| key_images.contains(key_image.as_ref()))?;

            if let Some(tx_hash) = tx_hash {
                txs.push(tx_hash.as_ref().try_into().unwrap());
            }
        }
    }

    Ok(txs)
}

/// Maps an input to a key image.
///
/// # Panics
/// This function will panic if the [`Input`] is not [`Input::ToKey`]
pub(super) fn ki_from_input(input: &Input) -> [u8; 32] {
    match input {
        Input::ToKey { key_image, .. } => key_image.to_bytes(),
        Input::Gen(_) => panic!("miner tx cannot be added to the txpool"),
    }
}
