//! Block functions.

use std::cmp::min;
use std::collections::HashMap;
use std::io;
//---------------------------------------------------------------------------------------------------- Import
use std::io::Write;

use bytemuck::TransparentWrapper;
use bytes::Bytes;
use fjall::Readable;
use cuprate_helper::cast::usize_to_u64;
use cuprate_helper::{
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
    tx::tx_fee,
};
use cuprate_pruning::CRYPTONOTE_PRUNING_LOG_STRIPES;
use cuprate_types::{
    AltBlockInformation, BlockCompleteEntry, ChainId, ExtendedBlockHeader, HardFork,
    PrunedTxBlobEntry, TransactionBlobs, VerifiedBlockInformation, VerifiedTransactionInformation,
};
use monero_oxide::{
    block::{Block, BlockHeader},
    transaction::Transaction,
};

use crate::database::{
    BLOCK_INFOS, PRUNABLE_BLOBS, PRUNED_BLOBS, RCT_OUTPUTS, TX_INFOS,
    V1_PRUNABLE_BLOBS,
};
use crate::error::{BlockchainError, DbResult};
use crate::Blockchain;
use crate::ops::tx::{add_tx_to_dynamic_tables, add_tx_to_tapes, remove_tx_from_dynamic_tables};
use crate::types::{Amount, RctOutput, TxInfo};
use crate::{
    ops::{alt_block, blockchain::chain_height, macros::doc_error},
    types::{BlockHash, BlockHeight, BlockInfo},
};

//---------------------------------------------------------------------------------------------------- `add_block_*`
pub fn add_blocks_to_tapes(
    blocks: &[VerifiedBlockInformation],
    db: &Blockchain,
    append_tx: &mut tapes::TapesAppendTransaction,
) -> DbResult<()> {
    let mut pruned_tape_index = u64::MAX;
    for block in blocks {
        pruned_tape_index = min(append_tx.append_bytes(&db.pruned_blobs, &block.block_blob)?, pruned_tape_index);
        
        for tx in &block.txs {
            append_tx.append_bytes(&db.pruned_blobs, tx.tx_pruned.as_slice())?;
            
            let prunable_hash =
                if tx.tx_prunable_blob.is_empty() || tx.tx.version() == 1 {
                    [0; 32]
                } else {
                    monero_oxide::primitives::keccak256(&tx.tx_prunable_blob)
                };
            append_tx.append_bytes(&db.pruned_blobs, &prunable_hash)?;
        }
    }

    let mut write_v2_prunable_data = |append_tx: &mut tapes::TapesAppendTransaction, tape, blocks: &[VerifiedBlockInformation]| -> io::Result<u64> {
       let mut first_idx = u64::MAX;
        for block in blocks {
            for tx in &block.txs {
                if tx.tx.version() != 1 {
                    first_idx = min(append_tx.append_bytes(tape, tx.tx_prunable_blob.as_slice())?, first_idx);
                }
            }
        }
        if first_idx == u64::MAX {
            first_idx = 0;
        }

        Ok(first_idx)
    };
    
    let mut write_v1_prunable_data = |append_tx: &mut tapes::TapesAppendTransaction, blocks: &[VerifiedBlockInformation]| -> io::Result<u64> {
        let mut first_idx = u64::MAX;
        for block in blocks {
            for tx in &block.txs {
                if tx.tx.version() == 1 {
                    let mut first_idx = min(append_tx.append_bytes(&db.v1_prunable_blobs, &tx.tx_prunable_blob)?, first_idx);
                }
            }
        }

        if first_idx == u64::MAX {
            first_idx = 0;
        }
        
        Ok(first_idx)
    };
    
    // Split the blocks at the point the pruning stripe changes.
    let start_height = blocks[0].height;
    let first_block_pruning_seed = cuprate_pruning::DecompressedPruningSeed::new(
        cuprate_pruning::get_block_pruning_stripe(start_height, usize::MAX, 3).unwrap(),
        3,
    )
    .unwrap();
    let next_stripe_height = first_block_pruning_seed
        .get_next_pruned_block(start_height, 500_000_000)
        .unwrap()
        .unwrap();

    let (first_stripe, next_stripe) =
        blocks.split_at(min(next_stripe_height - start_height, blocks.len()));

    for blocks in [first_stripe, next_stripe] {
        if blocks.is_empty() {
            continue;
        }

        let stripe =
            cuprate_pruning::get_block_pruning_stripe(blocks[0].height, usize::MAX, 3).unwrap();
        let mut v2_prunable_index = write_v2_prunable_data(append_tx, &db.prunable_blobs[stripe as usize - 1], &blocks)?;

        let mut v1_prunable_index = write_v1_prunable_data(append_tx, &blocks)?;

        let mut numb_rct_outs = append_tx
            .fixed_sized_tape_len(&db.rct_outputs).expect("Required tape not found");

        for block in blocks {
            let block_pruned_blob_idx = pruned_tape_index;
            let block_v1_prunable_idx = v1_prunable_index;
            let block_v2_prunable_idx = v2_prunable_index;

            let header_len = block.block.header.serialize().len() as u64;

            let mining_tx_index = {
                let tx = block.block.miner_transaction();
                add_tx_to_tapes(
                    &tx.clone().into(),
                    pruned_tape_index + header_len,
                    0,
                    tx.serialize().len() as u64,
                    0,
                    &block.height,
                    &mut numb_rct_outs,
                    append_tx,
                    db
                )?
            };

            pruned_tape_index += block.block_blob.len() as u64;

            for tx in &block.txs {
                add_tx_to_tapes(
                    &tx.tx,
                    pruned_tape_index,
                    if tx.tx.version() == 1 {
                        v1_prunable_index
                    } else {
                        v2_prunable_index
                    },
                    tx.tx_pruned.len() as u64,
                    tx.tx_prunable_blob.len() as u64,
                    &block.height,
                    &mut numb_rct_outs,
                    append_tx,
                    db
                )?;

                pruned_tape_index += tx.tx_pruned.len() as u64 + 32;
                if tx.tx.version() == 1 {
                    v1_prunable_index += tx.tx_prunable_blob.len() as u64;
                } else {
                    v2_prunable_index += tx.tx_prunable_blob.len() as u64;
                }
            }

            // `saturating_add` is used here as cumulative generated coins overflows due to tail emission.
            let cumulative_generated_coins = if let Some(prev_height) = block.height.checked_sub(1) {
                let mut prev_block_info = BlockInfo::default();
                append_tx.read_entries(&db.block_infos, prev_height as u64, &mut [prev_block_info]).unwrap();
                prev_block_info.cumulative_generated_coins.saturating_add(block.generated_coins)
            } else {
                block.generated_coins
            };

            let (cumulative_difficulty_low, cumulative_difficulty_high) =
                split_u128_into_low_high_bits(block.cumulative_difficulty);

            append_tx.append_entries(&db.block_infos,&[BlockInfo {
                cumulative_difficulty_low,
                cumulative_difficulty_high,
                cumulative_generated_coins,
                cumulative_rct_outs: numb_rct_outs,
                block_hash: block.block_hash,
                weight: block.weight,
                long_term_weight: block.long_term_weight,
                mining_tx_index,
                pruned_blob_idx: block_pruned_blob_idx,
                v1_prunable_blob_idx: block_v1_prunable_idx,
                prunable_blob_idx: block_v2_prunable_idx,
            }])?;
        }
    }

    Ok(())
}

/// Add a [`VerifiedBlockInformation`] to the database.
///
/// This extracts all the data from the input block and
/// maps/adds them to the appropriate database tables.
///
#[doc = doc_error!()]
///
/// # Panics
/// This function will panic if:
/// - `block.height > u32::MAX` (not normally possible)
/// - `block.height` is != [`chain_height`]
// no inline, too big.
pub fn add_block_to_dynamic_tables(
    db: &Blockchain,
    block: &VerifiedBlockInformation,
    numb_transactions: &mut u64,
    tx_rw: &mut fjall::SingleWriterWriteTx,
    pre_rct_numb_outputs_cache: &mut HashMap<Amount, u64>,
) -> DbResult<()> {
    //------------------------------------------------------ Check preconditions first

    // Cast height to `u32` for storage (handled at top of function).
    // Panic (should never happen) instead of allowing DB corruption.
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1560020991>
    assert!(
        u32::try_from(block.height).is_ok(),
        "block.height ({}) > u32::MAX",
        block.height,
    );

    let chain_height = tx_rw.last_key_value(&db.block_heights_fjall).map_or(0, |kv| usize::from_le_bytes(kv.value().unwrap().as_ref().try_into().unwrap()));

    //------------------------------------------------------ Transaction / Outputs / Key Images
    // Add the miner transaction first.
    let tx = block.block.miner_transaction();
    add_tx_to_dynamic_tables(
        db,
        &tx.clone().into(),
        *numb_transactions,
        &tx.hash(),
        &chain_height,
        tx_rw,
        pre_rct_numb_outputs_cache,
    )?;
    *numb_transactions += 1;

    for tx in &block.txs {
        add_tx_to_dynamic_tables(
            db,
            &tx.tx,
            *numb_transactions,
            &tx.tx_hash,
            &chain_height,
            tx_rw,
            pre_rct_numb_outputs_cache
        )?;
        *numb_transactions += 1;
    }

    tx_rw.insert(&db.block_heights_fjall, &block.block_hash, &block.height.to_le_bytes());

    Ok(())
}


//---------------------------------------------------------------------------------------------------- `pop_block`
/// Remove the top/latest block from the database.
///
/// The removed block's data is returned.
///
/// If a [`ChainId`] is specified the popped block will be added to the alt block tables under
/// that [`ChainId`]. Otherwise, the block will be completely removed from the DB.
#[doc = doc_error!()]
///
/// In `pop_block()`'s case, [`RuntimeError::KeyNotFound`]
/// will be returned if there are no blocks left.
// no inline, too big
pub fn pop_block(
    db: &Blockchain,
    move_to_alt_chain: Option<ChainId>,
    tx_rw: &mut fjall::SingleWriterWriteTx,
    tapes: &mut (),
) -> DbResult<(BlockHeight, BlockHash, Block)> {
    todo!()
    /*
    //------------------------------------------------------ Block Info
    let mut block_info_tape = tapes.fixed_sized_tape_popper::<BlockInfo>(BLOCK_INFOS);

    // Remove block data from tables.
    let (block_height, &block_info) = block_info_tape
        .pop_last()
        .ok_or(BlockchainError::NotFound)?;

    tx_rw.remove(&db.block_heights_fjall, &block_info.block_hash);

    drop(block_info_tape);
    // Block blobs.
    //
    // We deserialize the block header blob and mining transaction blob
    // to form a `Block`, such that we can remove the associated transactions
    // later.
    let pruned_blobs = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let block = Block::read(&mut pruned_blobs.get(block_info.pruned_blob_idx..).unwrap()).unwrap();

    drop(pruned_blobs);
    //------------------------------------------------------ Transaction / Outputs / Key Images
    remove_tx_from_dynamic_tables(
        db,
        &block.miner_transaction().hash(),
        block_height,
        tx_rw,
        tapes,
    )?;

    let remove_tx_iter = block.transactions.iter().map(|tx_hash| {
        let (_, tx) = remove_tx_from_dynamic_tables(db, tx_hash, block_height, tx_rw, tapes)?;
        Ok::<_, BlockchainError>(tx)
    });

    if let Some(chain_id) = move_to_alt_chain {
        let txs = remove_tx_iter
            .map(|result| {
                let tx = result?;
                let tx_weight = tx.weight();
                let tx_hash = tx.hash();
                let fee = tx_fee(&tx);
                let (tx_pruned, prunable) = tx.pruned_with_prunable();

                Ok(VerifiedTransactionInformation {
                    tx_weight,
                    tx_pruned: tx_pruned.serialize(),
                    tx_prunable_blob: prunable,
                    tx_hash,
                    fee,
                    tx: tx_pruned,
                })
            })
            .collect::<DbResult<Vec<VerifiedTransactionInformation>>>()?;

        alt_block::add_alt_block(
            db,
            &AltBlockInformation {
                block: block.clone(),
                block_blob: block.serialize(),
                txs,
                block_hash: block_info.block_hash,
                // We know the PoW is valid for this block so just set it so it will always verify as valid.
                pow_hash: [0; 32],
                height: block_height,
                weight: block_info.weight,
                long_term_weight: block_info.long_term_weight,
                cumulative_difficulty: combine_low_high_bits_to_u128(
                    block_info.cumulative_difficulty_low,
                    block_info.cumulative_difficulty_high,
                ),
                chain_id,
            },
            tx_rw,
        )?;
    } else {
        for result in remove_tx_iter {
            drop(result?);
        }
    }

    tapes
        .blob_tape_popper(PRUNED_BLOBS)
        .set_new_len(block_info.pruned_blob_idx);
    tapes
        .blob_tape_popper(V1_PRUNABLE_BLOBS)
        .set_new_len(block_info.v1_prunable_blob_idx);
    let stripe = cuprate_pruning::get_block_pruning_stripe(block_height, usize::MAX, 3).unwrap();
    tapes
        .blob_tape_popper(PRUNABLE_BLOBS[stripe as usize - 1])
        .set_new_len(block_info.prunable_blob_idx);

    tapes
        .fixed_sized_tape_popper::<TxInfo>(TX_INFOS)
        .set_new_len(block_info.mining_tx_index);

    let cumulative_rct_outs = tapes
        .fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS)
        .get(block_height - 1)
        .map_or(0, |info| info.cumulative_rct_outs);

    tapes
        .fixed_sized_tape_popper::<RctOutput>(RCT_OUTPUTS)
        .set_new_len(cumulative_rct_outs as usize);

    Ok((block_height, block_info.block_hash, block))
    
     */
}

//---------------------------------------------------------------------------------------------------- `get_block_complete_entry_*`
/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry(
    db: &Blockchain,
    block_hash: &BlockHash,
    pruned: bool,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<BlockCompleteEntry> {
    let block_height = tx_ro.get(&db.block_heights_fjall, &block_hash).expect("TODO")
        .ok_or(BlockchainError::NotFound)?;
    get_block_complete_entry_from_height(usize::from_le_bytes( block_height.as_ref().try_into().unwrap()), pruned, tapes, db)
}

/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry_from_height(
    block_height: BlockHeight,
    pruned: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &Blockchain,
) -> DbResult<BlockCompleteEntry> {
    let pruning_stripe = cuprate_pruning::get_block_pruning_stripe(
        block_height,
        usize::MAX,
        CRYPTONOTE_PRUNING_LOG_STRIPES,
    )
    .unwrap();

    let mut block_info = tapes.read_entry(&db.block_infos, block_height as u64)?.ok_or(BlockchainError::NotFound)?;

    let block_blob_start_idx = block_info.pruned_blob_idx;
    let mut block_blob_end_idx = None;

    let mut txs = Vec::with_capacity(32);

    for tx_info in tapes.iter_from(&db.tx_infos, block_info.mining_tx_index + 1)? {
        let tx_info = tx_info?;

        if tx_info.height != block_height {
            break;
        }

        block_blob_end_idx.get_or_insert(tx_info.pruned_blob_idx);

        txs.push(tx_info);
    }


    let txs = if txs.is_empty() {
        TransactionBlobs::None
    } else if pruned {
        let first_blob_idx = txs.first().unwrap().pruned_blob_idx;
        let mut blob = vec![0; (txs.last().unwrap().pruned_blob_idx - first_blob_idx  + txs.last().unwrap().pruned_size + 32) as usize];
        tapes.read_bytes(&db.pruned_blobs, first_blob_idx, &mut blob)?;

        let mut bytes = Bytes::from(blob);

        TransactionBlobs::Pruned(
            txs.into_iter()
                .map(|tx_info| PrunedTxBlobEntry {
                    blob: bytes.split_to(tx_info.pruned_size as usize),
                    prunable_hash: bytes.split_to(32).try_into().unwrap(),
                })
                .collect(),
        )
    } else {
        TransactionBlobs::Normal(
            txs.into_iter()
                .map(|tx_info| {
                    let mut blob = vec![0; (tx_info.pruned_size + tx_info.prunable_size) as usize];

                    tapes.read_bytes(&db.pruned_blobs, tx_info.pruned_blob_idx, &mut blob[..tx_info.pruned_size as usize])?;
                    if tx_info.rct_output_start_idx == u64::MAX {
                        tapes.read_bytes(&db.v1_prunable_blobs, tx_info.prunable_blob_idx, &mut blob[(tx_info.pruned_size as usize)..])?;
                    } else {
                        tapes.read_bytes(&db.prunable_blobs[pruning_stripe as usize - 1], tx_info.prunable_blob_idx, &mut blob[(tx_info.pruned_size as usize)..])?;
                    }

                    Ok(Bytes::from(blob))
                })
                .collect::<Result<_, BlockchainError>>()?,
        )
    };

    let block_blob = {
        let block_blob_end_idx = block_blob_end_idx.map(Ok).unwrap_or_else(|| {
            let next_block_info = tapes.read_entry(&db.block_infos, (block_height + 1) as u64)?;

            if let Some(info) = next_block_info {
                return Ok::<_, BlockchainError>(info.pruned_blob_idx);
            };

            Ok(tapes.blob_tape_len(&db.pruned_blobs).expect("Required tape not found"))
        })?;

        let mut blob = vec![0; (block_blob_end_idx - block_blob_start_idx) as usize];

        tapes.read_bytes(&db.pruned_blobs, block_blob_start_idx, &mut blob)?;

        Bytes::from(blob)
    };

    Ok(BlockCompleteEntry {
        block: block_blob,
        txs,
        pruned,
        block_weight: if pruned { block_info.weight as u64 } else { 0 },
    })
}

//---------------------------------------------------------------------------------------------------- `get_block_extended_header_*`
/// Retrieve a [`ExtendedBlockHeader`] from the database.
///
/// This extracts all the data from the database tables
/// needed to create a full `ExtendedBlockHeader`.
///
/// # Notes
/// This is slightly more expensive than [`get_block_extended_header_from_height`]
/// (1 more database lookup).
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header(
    db: &Blockchain,
    block_hash: &BlockHash,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<ExtendedBlockHeader> {
    let block_height = tx_ro.get(&db.block_heights_fjall, &block_hash).expect("TODO")
        .ok_or(BlockchainError::NotFound)?;

    get_block_extended_header_from_height(
        usize::from_le_bytes( block_height.as_ref().try_into().unwrap()),
        tapes,
        db
    )
}

/// Same as [`get_block_extended_header`] but with a [`BlockHeight`].
#[doc = doc_error!()]
#[expect(
    clippy::missing_panics_doc,
    reason = "The panic is only possible with a corrupt DB"
)]
#[inline]
pub fn get_block_extended_header_from_height(
    block_height: BlockHeight,
    tapes: &tapes::TapesReadTransaction,
    db: &Blockchain,
) -> DbResult<ExtendedBlockHeader> {
    let block_info = tapes
        .read_entry(&db.block_infos, block_height as u64)?.ok_or(BlockchainError::NotFound)?;
    let miner_tx_info = tapes
        .read_entry(&db.tx_infos, block_info.mining_tx_index)?.ok_or(BlockchainError::NotFound)?;

    let mut block_header_blob = vec![0; (miner_tx_info.pruned_blob_idx - block_info.pruned_blob_idx) as usize];
    tapes.read_bytes(&db.pruned_blobs, block_info.pruned_blob_idx, &mut block_header_blob)?;

    let block_header = BlockHeader::read(&mut block_header_blob.as_slice()).unwrap();

    let cumulative_difficulty = combine_low_high_bits_to_u128(
        block_info.cumulative_difficulty_low,
        block_info.cumulative_difficulty_high,
    );

    Ok(ExtendedBlockHeader {
        cumulative_difficulty,
        version: HardFork::from_version(block_header.hardfork_version)
            .expect("Stored block must have a valid hard-fork"),
        vote: block_header.hardfork_signal,
        timestamp: block_header.timestamp,
        block_weight: block_info.weight,
        long_term_weight: block_info.long_term_weight,
    })
}

/// Return the top/latest [`ExtendedBlockHeader`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header_top(
    db: &Blockchain,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<(ExtendedBlockHeader, BlockHeight)> {
    let height = tapes.fixed_sized_tape_len(&db.block_infos).expect("Require tape not found") as usize;
    let header = get_block_extended_header_from_height(height, tapes, db)?;
    Ok((header, height))

}

//---------------------------------------------------------------------------------------------------- Block
/// Retrieve a [`Block`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block(block_height: &BlockHeight, tapes: &tapes::TapesReadTransaction, db: &Blockchain) -> DbResult<Block> {
    let block_info = tapes.read_entry(&db.block_infos, *block_height as u64)?.ok_or(BlockchainError::NotFound)?;

    let pruned_end_blob_idx = match tapes.read_entry(&db.tx_infos, block_info.mining_tx_index + 1)? {
        Some(tx_info) if tx_info.height == *block_height => tx_info.pruned_blob_idx,
        Some(_) => tapes.read_entry(&db.block_infos, (*block_height + 1) as u64)?.ok_or(BlockchainError::NotFound)?.pruned_blob_idx,
        None => tapes.blob_tape_len(&db.pruned_blobs).expect("Required tape not found"),
    };

    let mut blob = vec![0; (pruned_end_blob_idx - block_info.pruned_blob_idx) as usize];

    tapes.read_bytes(&db.pruned_blobs, block_info.pruned_blob_idx, &mut blob)?;

    Ok(Block::read(
        &mut blob.as_slice(),
    )?)
}

/// Retrieve a [`Block`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_by_hash(
    db: &Blockchain,
    block_hash: &BlockHash,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<Block> {
    let block_height = tx_ro.get(&db.block_heights_fjall, &block_hash).expect("TODO")
        .ok_or(BlockchainError::NotFound)?;

    get_block(&usize::from_le_bytes( block_height.as_ref().try_into().unwrap()), tapes, db)
}

//---------------------------------------------------------------------------------------------------- Misc
/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(db: &Blockchain, block_hash: &BlockHash, tx_ro: &fjall::Snapshot) -> DbResult<BlockHeight> {
    let block_height = tx_ro.get(&db.block_heights_fjall, &block_hash).expect("TODO")
        .ok_or(BlockchainError::NotFound)?;

    Ok(usize::from_le_bytes( block_height.as_ref().try_into().unwrap()))
}

/// Check if a block exists in the database.
///
/// # Errors
/// Note that this will never return `Err(RuntimeError::KeyNotFound)`,
/// as in that case, `Ok(false)` will be returned.
///
/// Other errors may still occur.
#[inline]
pub fn block_exists(db: &Blockchain, block_hash: &BlockHash, tx_ro: &fjall::Snapshot) -> DbResult<bool> {
    Ok(tx_ro.contains_key(&db.block_heights_fjall, &block_hash).expect("TODO"))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[expect(clippy::too_many_lines)]
mod test {
    use pretty_assertions::assert_eq;

    use cuprate_database::{Env, EnvInner, TxRw};
    use cuprate_test_utils::data::{BLOCK_V16_TX0, BLOCK_V1_TX2, BLOCK_V9_TX3};

    use crate::{
        ops::tx::{get_tx, tx_exists},
        tables::OpenTables,
        tests::{assert_all_tables_are_empty, tmp_concrete_env, AssertTableLen},
    };

    use super::*;

    /// Tests all above block functions.
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    fn all_block_functions() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let mut blocks = [
            BLOCK_V1_TX2.clone(),
            BLOCK_V9_TX3.clone(),
            BLOCK_V16_TX0.clone(),
        ];
        // HACK: `add_block()` asserts blocks with non-sequential heights
        // cannot be added, to get around this, manually edit the block height.
        for (height, block) in blocks.iter_mut().enumerate() {
            block.height = height;
            assert_eq!(block.block.serialize(), block.block_blob);
        }
        let generated_coins_sum = blocks
            .iter()
            .map(|block| block.generated_coins)
            .sum::<u64>();

        // Add blocks.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for block in &blocks {
                // println!("add_block: {block:#?}");
                add_block(block, &mut tables).unwrap();
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        // Assert all reads are OK.
        let block_hashes = {
            let tx_ro = env_inner.tx_ro().unwrap();
            let tables = env_inner.open_tables(&tx_ro).unwrap();

            // Assert only the proper tables were added to.
            AssertTableLen {
                block_infos: 3,
                block_header_blobs: 3,
                block_txs_hashes: 3,
                block_heights: 3,
                key_images: 69,
                num_outputs: 41,
                pruned_tx_blobs: 0,
                prunable_hashes: 0,
                outputs: 111,
                prunable_tx_blobs: 0,
                rct_outputs: 8,
                tx_blobs: 8,
                tx_ids: 8,
                tx_heights: 8,
                tx_unlock_time: 3,
            }
            .assert(&tables);

            // Check `cumulative` functions work.
            assert_eq!(
                cumulative_generated_coins(&2, tables.block_infos()).unwrap(),
                generated_coins_sum,
            );

            // Both height and hash should result in getting the same data.
            let mut block_hashes = vec![];
            for block in &blocks {
                println!("blocks.iter(): hash: {}", hex::encode(block.block_hash));

                let height = get_block_height(&block.block_hash, tables.block_heights()).unwrap();

                println!("blocks.iter(): height: {height}");

                assert!(block_exists(&block.block_hash, tables.block_heights()).unwrap());

                let block_header_from_height =
                    get_block_extended_header_from_height(&height, &tables).unwrap();
                let block_header_from_hash =
                    get_block_extended_header(&block.block_hash, &tables).unwrap();

                // Just an alias, these names are long.
                let b1 = block_header_from_hash;
                let b2 = block;
                assert_eq!(b1, block_header_from_height);
                assert_eq!(b1.version.as_u8(), b2.block.header.hardfork_version);
                assert_eq!(b1.vote, b2.block.header.hardfork_signal);
                assert_eq!(b1.timestamp, b2.block.header.timestamp);
                assert_eq!(b1.cumulative_difficulty, b2.cumulative_difficulty);
                assert_eq!(b1.block_weight, b2.weight);
                assert_eq!(b1.long_term_weight, b2.long_term_weight);

                block_hashes.push(block.block_hash);

                // Assert transaction reads are OK.
                for (i, tx) in block.txs.iter().enumerate() {
                    println!("tx_hash: {:?}", hex::encode(tx.tx_hash));

                    assert!(tx_exists(&tx.tx_hash, tables.tx_ids()).unwrap());

                    let tx2 = get_tx(&tx.tx_hash, tables.tx_ids(), tables.tx_blobs()).unwrap();

                    assert_eq!(tx.tx_blob, tx2.serialize());
                    assert_eq!(tx.tx_weight, tx2.weight());
                    assert_eq!(tx.tx_hash, block.block.transactions[i]);
                    assert_eq!(tx.tx_hash, tx2.hash());
                }
            }

            block_hashes
        };

        {
            let len = block_hashes.len();
            let hashes: Vec<String> = block_hashes.iter().map(hex::encode).collect();
            println!("block_hashes: len: {len}, hashes: {hashes:?}");
        }

        // Remove the blocks.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for block_hash in block_hashes.into_iter().rev() {
                println!("pop_block(): block_hash: {}", hex::encode(block_hash));

                let (_popped_height, popped_hash, _popped_block) =
                    pop_block(None, &mut tables).unwrap();

                assert_eq!(block_hash, popped_hash);

                assert!(matches!(
                    get_block_extended_header(&block_hash, &tables),
                    Err(RuntimeError::KeyNotFound)
                ));
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }

    /// We should panic if: `block.height` > `u32::MAX`
    #[test]
    #[should_panic(expected = "block.height (4294967296) > u32::MAX")]
    fn block_height_gt_u32_max() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        let mut block = BLOCK_V9_TX3.clone();

        block.height = cuprate_helper::cast::u32_to_usize(u32::MAX) + 1;
        add_block(&block, &mut tables).unwrap();
    }

    /// We should panic if: `block.height` != the chain height
    #[test]
    #[should_panic(
        expected = "assertion `left == right` failed: block.height (123) != chain_height (1)\n  left: 123\n right: 1"
    )]
    fn block_height_not_chain_height() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        let mut block = BLOCK_V9_TX3.clone();
        // HACK: `add_block()` asserts blocks with non-sequential heights
        // cannot be added, to get around this, manually edit the block height.
        block.height = 0;

        // OK, `0 == 0`
        assert_eq!(block.height, 0);
        add_block(&block, &mut tables).unwrap();

        // FAIL, `123 != 1`
        block.height = 123;
        add_block(&block, &mut tables).unwrap();
    }
}
