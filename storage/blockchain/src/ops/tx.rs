//! Transaction functions.

use std::collections::HashMap;
//---------------------------------------------------------------------------------------------------- Import
use std::io::Read;

use crate::error::{BlockchainError, DbResult};
use crate::types::{Amount, TxInfo};
use crate::BlockchainDatabase;
use crate::{
    ops::{
        macros::{doc_add_block_inner_invariant, doc_error},
        output::{add_output, remove_output},
    },
    types::{BlockHeight, Output, PreRctOutputId, RctOutput, TxHash, TxId},
};
use bytemuck::TransparentWrapper;
use cuprate_helper::crypto::compute_zero_commitment;
use fjall::Readable;
use monero_oxide::transaction::{Input, Pruned, Timelock, Transaction};
use tapes::{TapesAppend, TapesRead, TapesTruncate};

//---------------------------------------------------------------------------------------------------- Private
///  TODO
#[expect(clippy::too_many_arguments)]
pub fn add_tx_to_tapes(
    tx: &Transaction<Pruned>,
    pruned_blob_idx: u64,
    prunable_blob_idx: u64,
    pruned_size: usize,
    prunable_size: usize,
    height: &BlockHeight,
    numb_rct_outputs: &mut u64,
    append_tx: &mut tapes::TapesAppendTransaction,
    db: &BlockchainDatabase,
) -> DbResult<TxId> {
    let tx_id = append_tx
        .fixed_sized_tape_len(&db.tx_infos)
        .expect("Required tape was not open.");

    append_tx.append_entries(
        &db.tx_infos,
        &[TxInfo {
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
        }],
    )?;

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

            append_tx.append_entries(
                &db.rct_outputs,
                &[RctOutput {
                    key: output.key.0,
                    height: *height,
                    timelock,
                    tx_idx: tx_id,
                    commitment: commitment.0,
                }],
            )?;

            *numb_rct_outputs += 1;
        }
    }

    Ok(tx_id)
}

pub fn add_tx_to_dynamic_tables(
    db: &BlockchainDatabase,
    tx: &Transaction<Pruned>,
    tx_id: TxId,
    tx_hash: &TxHash,
    height: &BlockHeight,
    w: &mut fjall::OwnedWriteBatch,
    pre_rct_numb_outputs_cache: &mut HashMap<Amount, u64>,
) -> DbResult<()> {
    w.insert(&db.tx_ids, tx_hash, tx_id.to_le_bytes());

    //------------------------------------------------------ Timelocks
    // Height/time is not differentiated via type, but rather:
    // "height is any value less than 500_000_000 and timestamp is any value above"
    // so the `u64/usize` is stored without any tag.
    //
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1558504285>
    let timelock = match tx.prefix().additional_timelock {
        Timelock::None => 0,
        Timelock::Block(height) => height as u64,
        Timelock::Time(time) => time,
    };

    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                w.insert(&db.key_images, key_image.as_bytes(), []);
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
                        &Output {
                            key: output.key.0,
                            height: *height,
                            timelock,
                            tx_idx: tx_id,
                        },
                        w,
                        pre_rct_numb_outputs_cache,
                    )?
                    .amount_index)
                })
                .collect::<DbResult<Vec<_>>>()?;

            w.insert(
                &db.v1_tx_outputs,
                tx_id.to_le_bytes(),
                bytemuck::cast_slice::<_, u8>(&amount_indices),
            );
        }
        Transaction::V2 { .. } => return Ok(()),
    }

    Ok(())
}

/// TODO
#[inline]
pub fn remove_tx_from_dynamic_tables(
    db: &BlockchainDatabase,
    tx_hash: &TxHash,
    height: BlockHeight,
    tx_rw: &mut fjall::OwnedWriteBatch,
    tapes: &tapes::TapesTruncateTransaction,
) -> DbResult<(TxId, Transaction)> {
    //------------------------------------------------------ Transaction data
    let tx_id = u64::from_le_bytes(
        db.tx_ids
            .get(tx_hash)
            .expect("TODO")
            .ok_or(BlockchainError::NotFound)?
            .as_ref()
            .try_into()
            .unwrap(),
    );

    //------------------------------------------------------ Unlock Time
    let tx_info = tapes.read_entry::<TxInfo>(&db.tx_infos, tx_id)?.unwrap();

    //------------------------------------------------------
    // Refer to the inner transaction type from now on.
    let tx = get_tx_from_id(&tx_id, tapes, db)?;

    //------------------------------------------------------ Key Images
    for inputs in &tx.prefix().inputs {
        match inputs {
            // Key images.
            Input::ToKey { key_image, .. } => {
                tx_rw.remove(&db.key_images, key_image.as_bytes());
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

        tx_rw.remove(&db.v1_tx_outputs, tx_id.to_le_bytes());
    }

    Ok((tx_id, tx))
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// Retrieve a [`Transaction`] from the database with its [`TxHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx(
    db: &BlockchainDatabase,
    tx_hash: &TxHash,
    tx_ro: &fjall::Snapshot,
    tapes: &impl tapes::TapesRead,
) -> DbResult<Transaction> {
    let tx_id = tx_ro
        .get(&db.tx_ids, tx_hash)
        .expect("TODO")
        .ok_or(BlockchainError::NotFound)?;

    get_tx_from_id(
        &u64::from_le_bytes(tx_id.as_ref().try_into().unwrap()),
        tapes,
        db,
    )
}

/// Retrieve a [`Transaction`] from the database with its [`TxId`].
#[doc = doc_error!()]
#[inline]
pub fn get_tx_from_id(
    tx_id: &TxId,
    tapes: &impl tapes::TapesRead,
    db: &BlockchainDatabase,
) -> DbResult<Transaction> {
    let blob = get_tx_blob_from_id(tx_id, tapes, db)?;
    let tx = Transaction::read(&mut blob.as_slice())?;

    Ok(tx)
}

pub fn get_tx_blob_from_id(
    tx_id: &TxId,
    tapes: &impl tapes::TapesRead,
    db: &BlockchainDatabase,
) -> DbResult<Vec<u8>> {
    let tx_info = tapes
        .read_entry(&db.tx_infos, *tx_id)?
        .ok_or(BlockchainError::NotFound)?;

    let prunable_tape = if tx_info.rct_output_start_idx == u64::MAX {
        &db.v1_prunable_blobs
    } else {
        let stripe =
            cuprate_pruning::get_block_pruning_stripe(tx_info.height, usize::MAX, 3).unwrap();
        &db.prunable_blobs[stripe as usize - 1]
    };

    let mut blob = vec![0; tx_info.pruned_size + tx_info.prunable_size];

    tapes.read_bytes(
        &db.pruned_blobs,
        tx_info.pruned_blob_idx,
        &mut blob[..tx_info.pruned_size],
    )?;
    tapes.read_bytes(
        prunable_tape,
        tx_info.prunable_blob_idx,
        &mut blob[tx_info.pruned_size..],
    )?;

    Ok(blob)
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
pub fn get_num_tx(db: &BlockchainDatabase, tx_ro: &fjall::Snapshot) -> DbResult<u64> {
    Ok(tx_ro.len(&db.tx_ids).expect("TODO") as u64)
}

//----------------------------------------------------------------------------------------------------
/// Check if a transaction exists in the database.
///
/// Returns `true` if it does, else `false`.
#[doc = doc_error!()]
#[inline]
pub fn tx_exists(
    db: &BlockchainDatabase,
    tx_hash: &TxHash,
    tx_ro: &fjall::Snapshot,
) -> DbResult<bool> {
    Ok(tx_ro.contains_key(&db.tx_ids, tx_hash).expect("TODO"))
}
