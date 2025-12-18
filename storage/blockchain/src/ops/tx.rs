//! Transaction functions.

//---------------------------------------------------------------------------------------------------- Import
use std::io::Read;

use bytemuck::TransparentWrapper;
use cuprate_database_service::RuntimeError;
use cuprate_helper::crypto::compute_zero_commitment;
use heed::PutFlags;
use monero_oxide::transaction::{Input, Pruned, Timelock, Transaction};
use tapes::MmapFile;

use crate::database::{
    PRUNABLE_BLOBS, PRUNED_BLOBS, RCT_OUTPUTS, TX_INFOS,
    V1_PRUNABLE_BLOBS,
};
use crate::error::{BlockchainError, DbResult};
use crate::Blockchain;
use crate::types::{TxInfo, ZeroKey};
use crate::{
    ops::{
        macros::{doc_add_block_inner_invariant, doc_error},
        output::{add_output, remove_output},
    },
    types::{BlockHeight, Output, PreRctOutputId, RctOutput, TxHash, TxId},
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
    height: &BlockHeight,
    numb_rct_outputs: &mut u64,
    tapes: &mut tapes::Appender<MmapFile>,
) -> DbResult<TxId> {
    let mut tx_info_appender = tapes.fixed_sized_tape_appender(TX_INFOS);
    let tx_id = tx_info_appender.len();

    tx_info_appender.slice_to_write(1)?[0] = TxInfo {
        height: *height,
        pruned_blob_idx,
        prunable_blob_idx,
        pruned_size,
        prunable_size,
        rct_output_start_idx: if tx.version() == 1 {
            u64::MAX
        } else {
            *numb_rct_outputs
        },
        numb_rct_outputs: tx.prefix().outputs.len(),
    };

    //------------------------------------------------------ Pruning
    // SOMEDAY: implement pruning after `monero-oxide` does.
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // SOMEDAY: what to store here? which table?
    // }

    let timelock = match tx.prefix().additional_timelock {
        Timelock::None => 0,
        Timelock::Block(height) => height as u64,
        Timelock::Time(time) => time,
    };

    //------------------------------------------------------ Key Images
    // Is this a miner transaction?
    // Which table we add the output data to depends on this.
    // <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/blockchain_db/blockchain_db.cpp#L212-L216>
    let miner_tx = matches!(tx.prefix().inputs.as_slice(), &[Input::Gen(_)]);

    if let Transaction::V2 { prefix, proofs } = &tx {
        let mut rct_output_appender = tapes.fixed_sized_tape_appender(RCT_OUTPUTS);

        let rct_mut_slice = rct_output_appender.slice_to_write(prefix.outputs.len())?;

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

            rct_mut_slice[i] = RctOutput {
                key: output.key.0,
                height: *height,
                timelock,
                tx_idx: tx_id,
                commitment: commitment.0,
            };

            *numb_rct_outputs += 1;
        }
    };

    Ok(tx_id)
}

pub fn add_tx_to_dynamic_tables(
    db: &Blockchain,
    tx: &Transaction<Pruned>,
    tx_id: TxId,
    tx_hash: &TxHash,
    height: &BlockHeight,
    tx_rw: &mut heed::RwTxn,
) -> DbResult<()> {
    db.tx_ids.put(tx_rw, tx_hash, &tx_id)?;

    //------------------------------------------------------ Timelocks
    // Height/time is not differentiated via type, but rather:
    // "height is any value less than 500_000_000 and timestamp is any value above"
    // so the `u64/usize` is stored without any tag.
    //
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1558504285>
    let time_lock = match tx.prefix().additional_timelock {
        Timelock::None => 0,
        Timelock::Block(height) => height as u64,
        Timelock::Time(time) => time,
    };

    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                db.key_images.put_with_flags(
                    tx_rw,
                    PutFlags::NO_DUP_DATA,
                    &ZeroKey,
                    key_image.as_bytes(),
                )?;
            }
            // This is a miner transaction.
            Input::Gen(_) => (),
        }
    }

    match &tx {
        Transaction::V1 { prefix, .. } => {
            let amount_indices = prefix
                .outputs
                .iter()
                .map(|output| {
                    // Pre-RingCT outputs.
                    Ok(add_output(
                        db,
                        output.amount.unwrap_or(0),
                        output.key.0,
                        *height,
                        time_lock,
                        tx_id,
                        tx_rw,
                    )?
                    .amount_index)
                })
                .collect::<DbResult<Vec<_>>>()?;

            db.tx_outputs.put_with_flags(
                tx_rw,
                PutFlags::APPEND,
                &tx_id,
                &amount_indices,
            )?;
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
    db: &Blockchain,
    tx_hash: &TxHash,
    height: BlockHeight,
    tx_rw: &mut heed::RwTxn,
    tapes: &tapes::Popper<MmapFile>,
) -> DbResult<(TxId, Transaction)> {
    //------------------------------------------------------ Transaction data
    let tx_id = db.tx_ids
        .get(tx_rw, tx_hash)?
        .ok_or(BlockchainError::NotFound)?;

    //------------------------------------------------------ Unlock Time
    let tx_info = *tapes
        .fixed_sized_tape_slice::<TxInfo>(TX_INFOS)
        .get(tx_id)
        .unwrap();

    let pruned_tape = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        tapes.blob_tape_tape_reader(V1_PRUNABLE_BLOBS)
    } else {
        let stripe = cuprate_pruning::get_block_pruning_stripe(height, usize::MAX, 3).unwrap();
        tapes.blob_tape_tape_reader(PRUNABLE_BLOBS[stripe as usize - 1])
    };

    let mut pruned = pruned_tape
        .get(tx_info.pruned_blob_idx..tx_info.pruned_blob_idx + tx_info.pruned_size)
        .unwrap();
    let mut prunable = prunable_tape
        .get(tx_info.prunable_blob_idx..tx_info.prunable_blob_idx + tx_info.prunable_size)
        .unwrap();

    //------------------------------------------------------
    // Refer to the inner transaction type from now on.
    let tx = Transaction::read(&mut (&mut pruned).chain(&mut prunable))?;

    //------------------------------------------------------ Key Images
    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                db.key_images.delete_one_duplicate(
                    tx_rw,
                    &ZeroKey,
                    key_image.as_bytes(),
                )?;
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
    if tx.version() == 1 {
        for output in &tx.prefix().outputs {
            // Outputs with clear amounts.
            if let Some(amount) = output.amount {
                remove_output(db, amount, tx_rw)?;
            }
        }

        db.tx_outputs.delete(tx_rw, &tx_id)?;
    }

    Ok((tx_id, tx))
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// Retrieve a [`Transaction`] from the database with its [`TxHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx(
    db: &Blockchain,
    tx_hash: &TxHash,
    tx_ro: &heed::RoTxn,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<Transaction> {
    get_tx_from_id(
        &db.tx_ids
            .get(tx_ro, tx_hash)?
            .ok_or(BlockchainError::NotFound)?,
        tapes,
    )
}

/// Retrieve a [`Transaction`] from the database with its [`TxId`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx_from_id(tx_id: &TxId, tapes: &tapes::Reader<MmapFile>) -> DbResult<Transaction> {
    let tx_info = *tapes
        .fixed_sized_tape_slice::<TxInfo>(TX_INFOS)
        .get(*tx_id)
        .unwrap();

    let pruned_tape = tapes.blob_tape_tape_slice(PRUNED_BLOBS);

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        tapes.blob_tape_tape_slice(V1_PRUNABLE_BLOBS)
    } else {
        let stripe =
            cuprate_pruning::get_block_pruning_stripe(tx_info.height, usize::MAX, 3).unwrap();
        tapes.blob_tape_tape_slice(PRUNABLE_BLOBS[stripe as usize - 1])
    };

    let mut pruned =
        &pruned_tape[tx_info.pruned_blob_idx..(tx_info.pruned_blob_idx + tx_info.pruned_size)];
    let mut prunable = &prunable_tape
        [tx_info.prunable_blob_idx..(tx_info.prunable_blob_idx + tx_info.prunable_size)];

    let tx = Transaction::read(&mut (&mut pruned).chain(&mut prunable))?;

    Ok(tx)
}

pub fn get_tx_blob_from_id(tx_id: &TxId, tapes: &tapes::Reader<MmapFile>) -> DbResult<Vec<u8>> {
    let tx_info = *tapes
        .fixed_sized_tape_slice::<TxInfo>(TX_INFOS)
        .get(*tx_id)
        .ok_or(BlockchainError::NotFound)?;

    let pruned_tape = tapes.blob_tape_tape_slice(PRUNED_BLOBS);

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        tapes.blob_tape_tape_slice(V1_PRUNABLE_BLOBS)
    } else {
        let stripe =
            cuprate_pruning::get_block_pruning_stripe(tx_info.height, usize::MAX, 3).unwrap();
        tapes.blob_tape_tape_slice(PRUNABLE_BLOBS[stripe as usize - 1])
    };

    let mut pruned =
        &pruned_tape[tx_info.pruned_blob_idx..(tx_info.pruned_blob_idx + tx_info.pruned_size)];
    let mut prunable = &prunable_tape
        [tx_info.prunable_blob_idx..(tx_info.prunable_blob_idx + tx_info.prunable_size)];

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
pub fn get_num_tx(db: &Blockchain, tx_ro: &heed::RoTxn) -> DbResult<u64> {
    Ok(db.tx_ids.len(tx_ro)?)
}

//----------------------------------------------------------------------------------------------------
/// Check if a transaction exists in the database.
///
/// Returns `true` if it does, else `false`.
#[doc = doc_error!()]
#[inline]
pub fn tx_exists(db: &Blockchain, tx_hash: &TxHash, tx_ro: &heed::RoTxn) -> DbResult<bool> {
    Ok(db.tx_ids.get(tx_ro, tx_hash)?.is_some())
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
