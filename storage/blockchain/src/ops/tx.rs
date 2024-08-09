//! Transaction functions.

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::TransparentWrapper;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, Scalar};
use monero_serai::transaction::{Input, Timelock, Transaction};

use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError, StorableVec};

use crate::{
    ops::{
        key_image::{add_key_image, remove_key_image},
        macros::{doc_add_block_inner_invariant, doc_error},
        output::{
            add_output, add_rct_output, get_rct_num_outputs, remove_output, remove_rct_output,
        },
    },
    tables::{TablesMut, TxBlobs, TxIds},
    types::{BlockHeight, Output, OutputFlags, PreRctOutputId, RctOutput, TxHash, TxId},
};

//---------------------------------------------------------------------------------------------------- Private
/// Add a [`Transaction`] (and related data) to the database.
///
/// The `block_height` is the block that this `tx` belongs to.
///
/// Note that the caller's input is trusted implicitly and no checks
/// are done (in this function) whether the `block_height` is correct or not.
///
#[doc = doc_add_block_inner_invariant!()]
///
/// # Notes
/// This function is different from other sub-functions and slightly more similar to
/// [`add_block()`](crate::ops::block::add_block) in that it calls other sub-functions.
///
/// This function calls:
/// - [`add_output()`]
/// - [`add_rct_output()`]
/// - [`add_key_image()`]
///
/// Thus, after [`add_tx`], those values (outputs and key images)
/// will be added to database tables as well.
///
/// # Panics
/// This function will panic if:
/// - `block.height > u32::MAX` (not normally possible)
#[doc = doc_error!()]
#[inline]
pub fn add_tx(
    tx: &Transaction,
    tx_blob: &Vec<u8>,
    tx_hash: &TxHash,
    block_height: &BlockHeight,
    tables: &mut impl TablesMut,
) -> Result<TxId, RuntimeError> {
    let tx_id = get_num_tx(tables.tx_ids_mut())?;

    //------------------------------------------------------ Transaction data
    tables.tx_ids_mut().put(tx_hash, &tx_id)?;
    tables.tx_heights_mut().put(&tx_id, block_height)?;
    tables
        .tx_blobs_mut()
        .put(&tx_id, StorableVec::wrap_ref(tx_blob))?;

    //------------------------------------------------------ Timelocks
    // Height/time is not differentiated via type, but rather:
    // "height is any value less than 500_000_000 and timestamp is any value above"
    // so the `u64/usize` is stored without any tag.
    //
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1558504285>
    match tx.prefix().additional_timelock {
        Timelock::None => (),
        Timelock::Block(height) => tables.tx_unlock_time_mut().put(&tx_id, &(height as u64))?,
        Timelock::Time(time) => tables.tx_unlock_time_mut().put(&tx_id, &time)?,
    }

    //------------------------------------------------------ Pruning
    // SOMEDAY: implement pruning after `monero-serai` does.
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // SOMEDAY: what to store here? which table?
    // }

    //------------------------------------------------------
    let Ok(height) = u32::try_from(*block_height) else {
        panic!("add_tx(): block_height ({block_height}) > u32::MAX");
    };

    //------------------------------------------------------ Key Images
    // Is this a miner transaction?
    // Which table we add the output data to depends on this.
    // <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/blockchain_db/blockchain_db.cpp#L212-L216>
    let mut miner_tx = false;

    // Key images.
    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                add_key_image(key_image.compress().as_bytes(), tables.key_images_mut())?;
            }
            // This is a miner transaction, set it for later use.
            Input::Gen(_) => miner_tx = true,
        }
    }

    //------------------------------------------------------ Outputs
    // Output bit flags.
    // Set to a non-zero bit value if the unlock time is non-zero.
    let output_flags = match tx.prefix().additional_timelock {
        Timelock::None => OutputFlags::empty(),
        Timelock::Block(_) | Timelock::Time(_) => OutputFlags::NON_ZERO_UNLOCK_TIME,
    };

    let amount_indices = match &tx {
        Transaction::V1 { prefix, .. } => prefix
            .outputs
            .iter()
            .map(|output| {
                // Pre-RingCT outputs.
                Ok(add_output(
                    output.amount.unwrap_or(0),
                    &Output {
                        key: output.key.0,
                        height,
                        output_flags,
                        tx_idx: tx_id,
                    },
                    tables,
                )?
                .amount_index)
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?,
        Transaction::V2 { prefix, proofs } => prefix
            .outputs
            .iter()
            .enumerate()
            .map(|(i, output)| {
                // Create commitment.
                // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1559489302>
                // FIXME: implement lookup table for common values:
                // <https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/ringct/rctOps.cpp#L322>
                let commitment = if miner_tx {
                    ED25519_BASEPOINT_POINT
                        + *monero_serai::generators::H * Scalar::from(output.amount.unwrap_or(0))
                } else {
                    proofs
                        .as_ref()
                        .expect("A V2 transaction with no RCT proofs is a miner tx")
                        .base
                        .commitments[i]
                };

                // Add the RCT output.
                add_rct_output(
                    &RctOutput {
                        key: output.key.0,
                        height,
                        output_flags,
                        tx_idx: tx_id,
                        commitment: commitment.compress().0,
                    },
                    tables.rct_outputs_mut(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    tables
        .tx_outputs_mut()
        .put(&tx_id, &StorableVec(amount_indices))?;

    Ok(tx_id)
}

/// Remove a transaction from the database with its [`TxHash`].
///
/// This returns the [`TxId`] and [`TxBlob`](crate::types::TxBlob) of the removed transaction.
///
#[doc = doc_add_block_inner_invariant!()]
///
/// # Notes
/// As mentioned in [`add_tx`], this function will call other sub-functions:
/// - [`remove_output()`]
/// - [`remove_rct_output()`]
/// - [`remove_key_image()`]
///
/// Thus, after [`remove_tx`], those values (outputs and key images)
/// will be remove from database tables as well.
///
#[doc = doc_error!()]
#[inline]
pub fn remove_tx(
    tx_hash: &TxHash,
    tables: &mut impl TablesMut,
) -> Result<(TxId, Transaction), RuntimeError> {
    //------------------------------------------------------ Transaction data
    let tx_id = tables.tx_ids_mut().take(tx_hash)?;
    let tx_blob = tables.tx_blobs_mut().take(&tx_id)?;
    tables.tx_heights_mut().delete(&tx_id)?;
    tables.tx_outputs_mut().delete(&tx_id)?;

    //------------------------------------------------------ Pruning
    // SOMEDAY: implement pruning after `monero-serai` does.
    // table_prunable_hashes.delete(&tx_id)?;
    // table_prunable_tx_blobs.delete(&tx_id)?;
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // SOMEDAY: what to remove here? which table?
    // }

    //------------------------------------------------------ Unlock Time
    match tables.tx_unlock_time_mut().delete(&tx_id) {
        Ok(()) | Err(RuntimeError::KeyNotFound) => (),
        // An actual error occurred, return.
        Err(e) => return Err(e),
    }

    //------------------------------------------------------
    // Refer to the inner transaction type from now on.
    let tx = Transaction::read(&mut tx_blob.0.as_slice())?;

    //------------------------------------------------------ Key Images
    // Is this a miner transaction?
    let mut miner_tx = false;
    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                remove_key_image(key_image.compress().as_bytes(), tables.key_images_mut())?;
            }
            // This is a miner transaction, set it for later use.
            Input::Gen(_) => miner_tx = true,
        }
    } // for each input

    //------------------------------------------------------ Outputs
    // Remove each output in the transaction.
    for output in &tx.prefix().outputs {
        // Outputs with clear amounts.
        if let Some(amount) = output.amount {
            // RingCT miner outputs.
            if miner_tx && tx.version() == 2 {
                let amount_index = get_rct_num_outputs(tables.rct_outputs())? - 1;
                remove_rct_output(&amount_index, tables.rct_outputs_mut())?;
            // Pre-RingCT outputs.
            } else {
                let amount_index = tables.num_outputs_mut().get(&amount)? - 1;
                remove_output(
                    &PreRctOutputId {
                        amount,
                        amount_index,
                    },
                    tables,
                )?;
            }
        // RingCT outputs.
        } else {
            let amount_index = get_rct_num_outputs(tables.rct_outputs())? - 1;
            remove_rct_output(&amount_index, tables.rct_outputs_mut())?;
        }
    } // for each output

    Ok((tx_id, tx))
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// Retrieve a [`Transaction`] from the database with its [`TxHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx(
    tx_hash: &TxHash,
    table_tx_ids: &impl DatabaseRo<TxIds>,
    table_tx_blobs: &impl DatabaseRo<TxBlobs>,
) -> Result<Transaction, RuntimeError> {
    get_tx_from_id(&table_tx_ids.get(tx_hash)?, table_tx_blobs)
}

/// Retrieve a [`Transaction`] from the database with its [`TxId`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx_from_id(
    tx_id: &TxId,
    table_tx_blobs: &impl DatabaseRo<TxBlobs>,
) -> Result<Transaction, RuntimeError> {
    let tx_blob = table_tx_blobs.get(tx_id)?.0;
    Ok(Transaction::read(&mut tx_blob.as_slice())?)
}

//----------------------------------------------------------------------------------------------------
/// How many [`Transaction`]s are there?
///
/// This returns the amount of transactions currently stored.
///
/// For example:
/// - 0 transactions exist => returns 0
/// - 1 transactions exist => returns 1
/// - 5 transactions exist => returns 5
/// - etc
#[doc = doc_error!()]
#[inline]
pub fn get_num_tx(table_tx_ids: &impl DatabaseRo<TxIds>) -> Result<u64, RuntimeError> {
    table_tx_ids.len()
}

//----------------------------------------------------------------------------------------------------
/// Check if a transaction exists in the database.
///
/// Returns `true` if it does, else `false`.
#[doc = doc_error!()]
#[inline]
pub fn tx_exists(
    tx_hash: &TxHash,
    table_tx_ids: &impl DatabaseRo<TxIds>,
) -> Result<bool, RuntimeError> {
    table_tx_ids.contains(tx_hash)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;

    use cuprate_database::{Env, EnvInner, TxRw};
    use cuprate_test_utils::data::{TX_V1_SIG0, TX_V1_SIG2, TX_V2_RCT3};

    use crate::{
        tables::{OpenTables, Tables},
        tests::{assert_all_tables_are_empty, tmp_concrete_env, AssertTableLen},
    };

    /// Tests all above tx functions when only inputting `Transaction` data (no Block).
    #[test]
    fn all_tx_functions() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        // Monero `Transaction`, not database tx.
        let txs = [&*TX_V1_SIG0, &*TX_V1_SIG2, &*TX_V2_RCT3];

        // Add transactions.
        let tx_ids = {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            let tx_ids = txs
                .iter()
                .map(|tx| {
                    println!("add_tx(): {tx:#?}");
                    add_tx(&tx.tx, &tx.tx_blob, &tx.tx_hash, &0, &mut tables).unwrap()
                })
                .collect::<Vec<TxId>>();

            drop(tables);
            TxRw::commit(tx_rw).unwrap();

            tx_ids
        };

        // Assert all reads of the transactions are OK.
        let tx_hashes = {
            let tx_ro = env_inner.tx_ro().unwrap();
            let tables = env_inner.open_tables(&tx_ro).unwrap();

            // Assert only the proper tables were added to.
            AssertTableLen {
                block_infos: 0,
                block_blobs: 0,
                block_heights: 0,
                key_images: 4, // added to key images
                pruned_tx_blobs: 0,
                prunable_hashes: 0,
                num_outputs: 9,
                outputs: 10, // added to outputs
                prunable_tx_blobs: 0,
                rct_outputs: 2,
                tx_blobs: 3,
                tx_ids: 3,
                tx_heights: 3,
                tx_unlock_time: 1, // only 1 has a timelock
            }
            .assert(&tables);

            // Both from ID and hash should result in getting the same transaction.
            let mut tx_hashes = vec![];
            for (i, tx_id) in tx_ids.iter().enumerate() {
                println!("tx_ids.iter(): i: {i}, tx_id: {tx_id}");

                let tx_get_from_id = get_tx_from_id(tx_id, tables.tx_blobs()).unwrap();
                let tx_hash = tx_get_from_id.hash();
                let tx_get = get_tx(&tx_hash, tables.tx_ids(), tables.tx_blobs()).unwrap();

                println!("tx_ids.iter(): tx_get_from_id: {tx_get_from_id:#?}, tx_get: {tx_get:#?}");

                assert_eq!(tx_get_from_id.hash(), tx_get.hash());
                assert_eq!(tx_get_from_id.hash(), txs[i].tx_hash);
                assert_eq!(tx_get_from_id, tx_get);
                assert_eq!(tx_get, txs[i].tx);
                assert!(tx_exists(&tx_hash, tables.tx_ids()).unwrap());

                tx_hashes.push(tx_hash);
            }

            tx_hashes
        };

        // Remove the transactions.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for tx_hash in tx_hashes {
                println!("remove_tx(): tx_hash: {tx_hash:?}");

                let (tx_id, _) = remove_tx(&tx_hash, &mut tables).unwrap();
                assert!(matches!(
                    get_tx_from_id(&tx_id, tables.tx_blobs()),
                    Err(RuntimeError::KeyNotFound)
                ));
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }
}
