//! Transaction functions.

//---------------------------------------------------------------------------------------------------- Import
use std::io::Read;

use bytemuck::TransparentWrapper;
use monero_oxide::transaction::{Input, Pruned, Timelock, Transaction};

use cuprate_database::{DatabaseRo, DatabaseRw, DbResult, RuntimeError, StorableVec};
use cuprate_helper::crypto::compute_zero_commitment;

use crate::database::{PRUNABLE_BLOBS, PRUNED_BLOBS, RCT_OUTPUTS, TX_INFOS, V1_PRUNABLE_BLOBS};
use crate::types::TxInfo;
use crate::{
    ops::{
        key_image::{add_key_image, remove_key_image},
        macros::{doc_add_block_inner_invariant, doc_error},
        output::{
            add_output, remove_output,
        },
    },
    tables::{TablesMut, TxIds},
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
pub fn add_tx_to_tapes(
    tx: &Transaction<Pruned>,
    pruned_blob_idx: usize,
    prunable_blob_idx: usize,
    pruned_size: usize,
    prunable_size: usize,
    block_height: &BlockHeight,
    numb_rct_outputs: &mut u64,
    tapes: &mut cuprate_linear_tapes::Appender,
) -> DbResult<TxId> {
    let mut tx_info_appender = tapes.fixed_sized_tape_appender(TX_INFOS);
    let tx_id = tx_info_appender.len();

    tx_info_appender.push_entries(&[TxInfo {
        height: *block_height,
        pruned_blob_idx,
        prunable_blob_idx,
        pruned_size,
        prunable_size,
        rct_output_start_idx: *numb_rct_outputs,
        numb_rct_outputs: tx.prefix().outputs.len(),
    }])?;

    let mut rct_output_appender = tapes.fixed_sized_tape_appender(RCT_OUTPUTS);

    //------------------------------------------------------ Pruning
    // SOMEDAY: implement pruning after `monero-oxide` does.
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
    let miner_tx = matches!(tx.prefix().inputs.as_slice(), &[Input::Gen(_)]);

    //------------------------------------------------------ Outputs
    // Output bit flags.
    // Set to a non-zero bit value if the unlock time is non-zero.
    let output_flags = match tx.prefix().additional_timelock {
        Timelock::None => OutputFlags::empty(),
        Timelock::Block(_) | Timelock::Time(_) => OutputFlags::NON_ZERO_UNLOCK_TIME,
    };

    if let Transaction::V2 { prefix, proofs } = &tx {
        for (i, output) in prefix.outputs.iter().enumerate() {
            // Create commitment.
            let commitment = if miner_tx {
                compute_zero_commitment(output.amount.unwrap_or(0))
            } else {
                proofs
                    .as_ref()
                    .expect("A V2 transaction with no RCT proofs is a miner tx")
                    .base
                    .commitments[i]
            };

            rct_output_appender.push_entries(&[RctOutput {
                key: output.key.0,
                height,
                output_flags,
                tx_idx: tx_id,
                commitment: commitment.0,
            }])?;
        }
    };

    Ok(tx_id)
}

pub fn add_tx_to_dynamic_tables(
    tx: &Transaction<Pruned>,
    tx_id: TxId,
    tx_hash: &TxHash,
    block_height: &BlockHeight,
    tables: &mut impl TablesMut,
) -> DbResult<()> {
    tables.tx_ids_mut().put(tx_hash, &tx_id)?;

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

    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                add_key_image(key_image.as_bytes(), tables.key_images_mut())?;
            }
            // This is a miner transaction.
            Input::Gen(_) => (),
        }
    }

    //------------------------------------------------------
    let Ok(height) = u32::try_from(*block_height) else {
        panic!("add_tx(): block_height ({block_height}) > u32::MAX");
    };

    let output_flags = match tx.prefix().additional_timelock {
        Timelock::None => OutputFlags::empty(),
        Timelock::Block(_) | Timelock::Time(_) => OutputFlags::NON_ZERO_UNLOCK_TIME,
    };

    match &tx {
        Transaction::V1 { prefix, .. } => {
            let amount_indices = prefix
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
                .collect::<DbResult<Vec<_>>>()?;

            tables
                .tx_outputs_mut()
                .put(&tx_id, &StorableVec(amount_indices))?;
        }
        Transaction::V2 { .. } => return Ok(()),
    };

    Ok(())
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
pub fn remove_tx_from_dynamic_tables(
    tx_hash: &TxHash,
    height: BlockHeight,
    tables: &mut impl TablesMut,
    tapes: &cuprate_linear_tapes::Popper,
) -> DbResult<(TxId, Transaction)> {
    //------------------------------------------------------ Transaction data
    let tx_id = tables.tx_ids_mut().take(tx_hash)?;

    //------------------------------------------------------ Unlock Time
    match tables.tx_unlock_time_mut().delete(&tx_id) {
        Ok(()) | Err(RuntimeError::KeyNotFound) => (),
        // An actual error occurred, return.
        Err(e) => return Err(e),
    }

    let tx_info = tapes
        .fixed_sized_tape_reader::<TxInfo>(TX_INFOS)
        .try_get(tx_id)
        .unwrap();

    let pruned_tape = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        tapes.blob_tape_tape_reader(V1_PRUNABLE_BLOBS)
    } else {
        let stripe = cuprate_pruning::get_block_pruning_stripe(height, usize::MAX, 3).unwrap();
        tapes.blob_tape_tape_reader(PRUNABLE_BLOBS[stripe as usize - 1])
    };

    let mut pruned = pruned_tape
        .try_get_slice(tx_info.pruned_blob_idx, tx_info.pruned_size)
        .unwrap();
    let mut prunable = prunable_tape
        .try_get_slice(tx_info.prunable_blob_idx, tx_info.prunable_size)
        .unwrap();

    //------------------------------------------------------
    // Refer to the inner transaction type from now on.
    let tx = Transaction::read(&mut (&mut pruned).chain(&mut prunable))?;

    //------------------------------------------------------ Key Images
    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                remove_key_image(key_image.as_bytes(), tables.key_images_mut())?;
            }
            // This is a miner transaction, set it for later use.
            Input::Gen(_) => (),
        }
    } // for each input

    if tx.version() != 1 {
        return Ok((tx_id, tx));
    }

    //------------------------------------------------------ Outputs
    // Remove each output in the transaction.
    for output in &tx.prefix().outputs {
        // Outputs with clear amounts.
        if let Some(amount) = output.amount {
            let amount_index = tables.num_outputs_mut().get(&amount)? - 1;
            remove_output(
                &PreRctOutputId {
                    amount,
                    amount_index,
                },
                tables,
            )?;
        }
    }

    tables.tx_outputs_mut().delete(&tx_id)?;

    Ok((tx_id, tx))
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// Retrieve a [`Transaction`] from the database with its [`TxHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx(
    tx_hash: &TxHash,
    table_tx_ids: &impl DatabaseRo<TxIds>,
    tapes: &cuprate_linear_tapes::Reader,
) -> DbResult<Transaction> {
    get_tx_from_id(&table_tx_ids.get(tx_hash)?, tapes)
}

/// Retrieve a [`Transaction`] from the database with its [`TxId`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx_from_id(tx_id: &TxId, tapes: &cuprate_linear_tapes::Reader) -> DbResult<Transaction> {
    let tx_info = tapes
        .fixed_sized_tape_reader::<TxInfo>(TX_INFOS)
        .try_get(*tx_id)
        .unwrap();

    let pruned_tape = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        tapes.blob_tape_tape_reader(V1_PRUNABLE_BLOBS)
    } else {
        let stripe =
            cuprate_pruning::get_block_pruning_stripe(tx_info.height, usize::MAX, 3).unwrap();
        tapes.blob_tape_tape_reader(PRUNABLE_BLOBS[stripe as usize - 1])
    };

    let mut pruned = pruned_tape
        .try_get_slice(tx_info.pruned_blob_idx, tx_info.pruned_size)
        .unwrap();
    let mut prunable = prunable_tape
        .try_get_slice(tx_info.prunable_blob_idx, tx_info.prunable_size)
        .unwrap();

    let tx = Transaction::read(&mut (&mut pruned).chain(&mut prunable))?;

    Ok(tx)
}

pub fn get_tx_blob_from_id(tx_id: &TxId, tapes: &cuprate_linear_tapes::Reader) -> DbResult<Vec<u8>> {
    let tx_info = tapes
        .fixed_sized_tape_reader::<TxInfo>(TX_INFOS)
        .try_get(*tx_id)
        .unwrap();

    let pruned_tape = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        tapes.blob_tape_tape_reader(V1_PRUNABLE_BLOBS)
    } else {
        let stripe =
            cuprate_pruning::get_block_pruning_stripe(tx_info.height, usize::MAX, 3).unwrap();
        tapes.blob_tape_tape_reader(PRUNABLE_BLOBS[stripe as usize - 1])
    };

    let mut pruned = pruned_tape
        .try_get_slice(tx_info.pruned_blob_idx, tx_info.pruned_size)
        .unwrap();
    let mut prunable = prunable_tape
        .try_get_slice(tx_info.prunable_blob_idx, tx_info.prunable_size)
        .unwrap();

    Ok([pruned, prunable].concat())
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
pub fn get_num_tx(table_tx_ids: &impl DatabaseRo<TxIds>) -> DbResult<u64> {
    table_tx_ids.len()
}

//----------------------------------------------------------------------------------------------------
/// Check if a transaction exists in the database.
///
/// Returns `true` if it does, else `false`.
#[doc = doc_error!()]
#[inline]
pub fn tx_exists(tx_hash: &TxHash, table_tx_ids: &impl DatabaseRo<TxIds>) -> DbResult<bool> {
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
                block_header_blobs: 0,
                block_txs_hashes: 0,
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
