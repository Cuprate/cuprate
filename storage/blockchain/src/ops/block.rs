//! Block functions.

use std::io::Write;
//---------------------------------------------------------------------------------------------------- Import
use bytemuck::TransparentWrapper;
use bytes::{BufMut, Bytes};
use cuprate_database::{
    DbResult, RuntimeError, StorableVec, {DatabaseRo, DatabaseRw},
};
use cuprate_helper::cast::usize_to_u64;
use cuprate_helper::{
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
    tx::tx_fee,
};
use cuprate_linear_tape::{Flush, LinearBlobTapeAppender, LinearTapes, ResizeNeeded};
use cuprate_pruning::CRYPTONOTE_PRUNING_LOG_STRIPES;
use cuprate_types::{
    AltBlockInformation, BlockCompleteEntry, ChainId, ExtendedBlockHeader, HardFork,
    PrunedTxBlobEntry, TransactionBlobs, VerifiedBlockInformation, VerifiedTransactionInformation,
};
use monero_oxide::transaction::{NotPruned, Pruned};
use monero_oxide::{
    block::{Block, BlockHeader},
    transaction::Transaction,
};
use parking_lot::RwLockUpgradableReadGuard;

use crate::database::{BLOCK_INFOS, PRUNABLE_BLOBS, PRUNED_BLOBS, TX_INFOS};
use crate::types::{RctOutput, TxInfo};
use crate::{
    ops::{
        alt_block,
        blockchain::chain_height,
        macros::doc_error,
        output::get_rct_num_outputs,
        tx::{add_tx, remove_tx},
    },
    tables::{BlockHeights, Tables, TablesIter, TablesMut},
    types::{BlockHash, BlockHeight, BlockInfo},
};

pub fn add_prunable_blocks_blobs(
    blocks: &[VerifiedBlockInformation],
    tape_appender: &mut cuprate_linear_tape::Appender,
) -> Result<usize, RuntimeError> {
    fn add_prunable_blocks_blobs_inner(
        blocks: &[VerifiedBlockInformation],
        prunable_blob_tape: &mut LinearBlobTapeAppender<'_>,
    ) -> Result<usize, ResizeNeeded> {
        struct PrunableTapeWriter<'a>(&'a [VerifiedBlockInformation]);

        impl cuprate_linear_tape::Blob for PrunableTapeWriter<'_> {
            fn len(&self) -> usize {
                self.0
                    .iter()
                    .map(|block| {
                        block
                            .txs
                            .iter()
                            .map(|tx| tx.tx_prunable_blob.len())
                            .sum::<usize>()
                    })
                    .sum::<usize>()
            }
            fn write(&self, buf: &mut [u8]) {
                let mut writer = buf.writer();

                for block in self.0 {
                    for blob in &block.txs {
                        writer.write_all(&blob.tx_prunable_blob).unwrap();
                    }
                }
            }
        }

        prunable_blob_tape.push_bytes(&PrunableTapeWriter(blocks))
    }

    let stripe =
        cuprate_pruning::get_block_pruning_stripe(blocks[0].height, usize::MAX, 3).unwrap();

    let mut pruned_appender = tape_appender.blob_tape_appender(PRUNABLE_BLOBS[stripe as usize - 1]);

    Ok(add_prunable_blocks_blobs_inner(blocks, &mut pruned_appender).unwrap())
}

pub fn add_pruned_blocks_blobs(
    blocks: &[VerifiedBlockInformation],
    tape_appender: &mut cuprate_linear_tape::Appender,
) -> Result<usize, RuntimeError> {
    fn add_pruned_blocks_blobs_inner(
        blocks: &[VerifiedBlockInformation],
        pruned_blob_tape: &mut LinearBlobTapeAppender<'_>,
    ) -> Result<usize, ResizeNeeded> {
        struct BlockTapeWriter<'a>(&'a [VerifiedBlockInformation]);

        impl cuprate_linear_tape::Blob for BlockTapeWriter<'_> {
            fn len(&self) -> usize {
                self.0
                    .iter()
                    .map(|block| {
                        block.block_blob.len()
                            + block
                                .txs
                                .iter()
                                .map(|tx| tx.tx_pruned.len() + 32)
                                .sum::<usize>()
                    })
                    .sum::<usize>()
            }
            fn write(&self, buf: &mut [u8]) {
                let mut writer = buf.writer();

                for block in self.0 {
                    writer.write_all(block.block_blob.as_slice()).unwrap();

                    for tx in &block.txs {
                        writer.write_all(tx.tx_pruned.as_slice()).unwrap();
                        let prunable_hash = if tx.tx_prunable_blob.is_empty() {
                            [0; 32]
                        } else {
                            monero_oxide::primitives::keccak256(&tx.tx_prunable_blob)
                        };
                        writer.write_all(&prunable_hash).unwrap();
                    }
                }
            }
        }

        pruned_blob_tape.push_bytes(&BlockTapeWriter(blocks))
    }

    let mut pruned_appender = tape_appender.blob_tape_appender(PRUNED_BLOBS);

    Ok(add_pruned_blocks_blobs_inner(blocks, &mut pruned_appender).unwrap())
}

//---------------------------------------------------------------------------------------------------- `add_block_*`
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
pub fn add_block(
    block: &VerifiedBlockInformation,
    pruned_tape_idx: &mut usize,
    prunable_tape_idx: &mut usize,
    numb_rct_outs: &mut u64,
    rct_outputs: &mut Vec<RctOutput>,
    tables: &mut impl TablesMut,
    tape_appender: &mut cuprate_linear_tape::Appender,
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

    let chain_height = chain_height(tables.block_heights())?;
    assert_eq!(
        block.height, chain_height,
        "block.height ({}) != chain_height ({})",
        block.height, chain_height,
    );

    // Expensive checks - debug only.
    #[cfg(debug_assertions)]
    {
        assert_eq!(block.block.serialize(), block.block_blob);
        assert_eq!(block.block.transactions.len(), block.txs.len());
        for (i, tx) in block.txs.iter().enumerate() {
            //assert_eq!(tx.tx_blob, tx.tx.serialize());
            assert_eq!(tx.tx_hash, block.block.transactions[i]);
        }
    }

    let block_blob_idx = *pruned_tape_idx;
    let header_len = block.block.header.serialize().len();

    let stripe = cuprate_pruning::get_block_pruning_stripe(
        block.height,
        usize::MAX,
        CRYPTONOTE_PRUNING_LOG_STRIPES,
    )
    .unwrap() as usize;

    //------------------------------------------------------ Transaction / Outputs / Key Images
    // Add the miner transaction first.
    let mining_tx_index = {
        let tx = block.block.miner_transaction();
        add_tx(
            &tx.clone().into(),
            *pruned_tape_idx + header_len,
            *prunable_tape_idx,
            tx.serialize().len(),
            0,
            &tx.hash(),
            &chain_height,
            numb_rct_outs,
            tables,
            rct_outputs,
            tape_appender,
        )?
    };

    *pruned_tape_idx += block.block_blob.len();
    for tx in &block.txs {
        add_tx(
            &tx.tx,
            *pruned_tape_idx,
            *prunable_tape_idx,
            tx.tx_pruned.len(),
            tx.tx_prunable_blob.len(),
            &tx.tx_hash,
            &chain_height,
            numb_rct_outs,
            tables,
            rct_outputs,
            tape_appender,
        )?;
        *pruned_tape_idx += tx.tx_pruned.len() + 32;
        *prunable_tape_idx += tx.tx_prunable_blob.len();
    }

    //------------------------------------------------------ Block Info

    let mut block_info_appender = tape_appender.fixed_sized_tape_appender::<BlockInfo>(BLOCK_INFOS);

    // `saturating_add` is used here as cumulative generated coins overflows due to tail emission.
    let cumulative_generated_coins = block_info_appender
        .try_get(block.height.saturating_sub(1))
        .map(|i| i.cumulative_generated_coins)
        .unwrap_or(0)
        .saturating_add(block.generated_coins);

    let (cumulative_difficulty_low, cumulative_difficulty_high) =
        split_u128_into_low_high_bits(block.cumulative_difficulty);

    // Block Info.
    block_info_appender
        .push_entries(&[BlockInfo {
            cumulative_difficulty_low,
            cumulative_difficulty_high,
            cumulative_generated_coins,
            cumulative_rct_outs: *numb_rct_outs,
            block_hash: block.block_hash,
            weight: block.weight,
            long_term_weight: block.long_term_weight,
            mining_tx_index,
            blob_idx: block_blob_idx,
        }])
        .unwrap();

    // Block heights.
    tables
        .block_heights_mut()
        .put(&block.block_hash, &block.height, false)?;

    let mut blob_ends = tables.blob_tape_ends().get(&1)?;

    blob_ends.pruned_tape = *pruned_tape_idx;
    blob_ends.prunable_tapes[stripe - 1] = *prunable_tape_idx;

    tables.blob_tape_ends_mut().put(&1, &blob_ends, false)?;

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
    move_to_alt_chain: Option<ChainId>,
    tables: &mut impl TablesMut,
    tapes: &LinearTapes,
) -> DbResult<(BlockHeight, BlockHash, Block)> {
    todo!()
    /*
    //------------------------------------------------------ Block Info
    // Remove block data from tables.
    let (block_height, block_info) = tables.block_infos_mut().last()?;

    let stripe = cuprate_pruning::get_block_pruning_stripe(
        block_height,
        usize::MAX,
        CRYPTONOTE_PRUNING_LOG_STRIPES,
    )
        .unwrap() as usize;


    // Block heights.

    // Block blobs.
    //
    // We deserialize the block header blob and mining transaction blob
    // to form a `Block`, such that we can remove the associated transactions
    // later.
    let block = get_block(tables, &tapes, &block_height)?;

    //------------------------------------------------------ Transaction / Outputs / Key Images
    let mut rct_outputs = 0;
    let (_,miner_tx_info,_) = remove_tx(&block.miner_transaction().hash(),  &mut rct_outputs,  tables, &tapes)?;

    let remove_tx_iter = block.transactions.iter().map(|tx_hash| {
        let (_,_, tx) = remove_tx(tx_hash, &mut rct_outputs, tables, &tapes)?;
        Ok::<_, RuntimeError>(tx)
    });

    if let Some(chain_id) = move_to_alt_chain {
        let txs = remove_tx_iter
            .map(|result| {
                let tx = result?;
                let tx_weight =  tx.weight();
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
            tables,
        )?;
    } else {
        for result in remove_tx_iter {
            drop(result?);
        }
    }

    tables.block_heights_mut().delete(&block_info.block_hash)?;

    tables.block_infos_mut().pop_last()?;

    let mut rct_popper = tapes.rct_outputs.popper();
    rct_popper.pop_entries(rct_outputs as usize);
    rct_popper.flush(Flush::Async)?;

    tapes.pruned_blobs.truncate(Flush::Async, block_info.blob_idx)?;
    tapes.prunable_tape[stripe -1].as_mut().unwrap().truncate(Flush::Async, miner_tx_info.prunable_blob_idx)?;

    tables.blob_tape_ends_mut().update(&1, |mut ends| {
        ends.pruned_tape = block_info.blob_idx;
        ends.prunable_tapes[stripe - 1] = miner_tx_info.prunable_blob_idx;
        Some(ends)
    })?;;

    Ok((block_height, block_info.block_hash, block))

     */
}

//---------------------------------------------------------------------------------------------------- `get_block_complete_entry_*`
/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry(
    block_hash: &BlockHash,
    pruned: bool,
    tables: &impl TablesIter,
    tapes: &cuprate_linear_tape::Reader,
) -> Result<BlockCompleteEntry, RuntimeError> {
    let block_height = tables.block_heights().get(block_hash)?;
    get_block_complete_entry_from_height(&block_height, pruned, tables, tapes)
}

/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry_from_height(
    block_height: &BlockHeight,
    pruned: bool,
    tables: &impl TablesIter,
    tapes: &cuprate_linear_tape::Reader,
) -> Result<BlockCompleteEntry, RuntimeError> {
    let pruning_stripe = cuprate_pruning::get_block_pruning_stripe(
        *block_height,
        usize::MAX,
        CRYPTONOTE_PRUNING_LOG_STRIPES,
    )
    .unwrap();

    let pruned_tape_reader = tapes.blob_tape_tape_reader(PRUNED_BLOBS);
    let block_info_tape_reader = tapes.fixed_sized_tape_reader::<BlockInfo>(BLOCK_INFOS);

    let prunable_tape_reader =
        tapes.blob_tape_tape_reader(PRUNABLE_BLOBS[pruning_stripe as usize - 1]);
    let tx_infos_reader = tapes.fixed_sized_tape_reader::<TxInfo>(TX_INFOS);

    let block_info = block_info_tape_reader
        .try_get(*block_height)
        .ok_or(RuntimeError::KeyNotFound)?;

    let block_blob_start_idx = block_info.blob_idx;
    let mut block_blob_end_idx = None;

    let mut tx_info1 = tx_infos_reader.try_get((block_info.mining_tx_index + 1));
    let mut next_tx_info = tx_infos_reader.try_get((block_info.mining_tx_index + 2));

    let mut txs = Vec::with_capacity(32);

    let mut next_tx_info_idx = block_info.mining_tx_index + 2;
    while let Some(tx_info) = tx_info1 {
        if tx_info.height != *block_height {
            break;
        }

        block_blob_end_idx.get_or_insert(tx_info.pruned_blob_idx);

        if pruned {
            let blob = pruned_tape_reader
                .try_get_slice(tx_info.pruned_blob_idx, tx_info.pruned_size)
                .unwrap();
            let prunable_hash = pruned_tape_reader
                .try_get_slice(tx_info.pruned_blob_idx + tx_info.pruned_size, 32)
                .unwrap();

            txs.push((blob, prunable_hash));
        } else {
            let pruned_blob = pruned_tape_reader
                .try_get_slice(tx_info.pruned_blob_idx, tx_info.pruned_size)
                .unwrap();
            let prunable_blob = prunable_tape_reader
                .try_get_slice(tx_info.prunable_blob_idx, tx_info.prunable_size)
                .unwrap();

            txs.push((pruned_blob, prunable_blob));
        }

        tx_info1 = next_tx_info;
        next_tx_info = tx_infos_reader.try_get((next_tx_info_idx + 1));
        next_tx_info_idx += 1;
    }

    let txs = if pruned {
        TransactionBlobs::Pruned(
            txs.into_iter()
                .map(|(pruned, prunable_hash)| PrunedTxBlobEntry {
                    blob: Bytes::copy_from_slice(pruned),
                    prunable_hash: Bytes::copy_from_slice(prunable_hash).try_into().unwrap(),
                })
                .collect(),
        )
    } else {
        TransactionBlobs::Normal(
            txs.into_iter()
                .map(|(pruned, prunable)| {
                    let buf = [pruned, prunable].concat();
                    Bytes::from(buf)
                })
                .collect(),
        )
    };

    let block_blob = {
        let block_blob_end_idx = block_blob_end_idx.unwrap_or_else(|| {
            let next_block_info = block_info_tape_reader.try_get((*block_height + 1));

            if let Some(info) = next_block_info {
                return info.blob_idx;
            };

            tables.blob_tape_ends().get(&1).unwrap().pruned_tape
        });

        let blob = pruned_tape_reader
            .try_get_range(block_blob_start_idx..block_blob_end_idx)
            .unwrap();

        Bytes::copy_from_slice(blob)
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
    block_hash: &BlockHash,
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
) -> DbResult<ExtendedBlockHeader> {
    get_block_extended_header_from_height(&tables.block_heights().get(block_hash)?, tables, tapes)
}

/// Same as [`get_block_extended_header`] but with a [`BlockHeight`].
#[doc = doc_error!()]
#[expect(
    clippy::missing_panics_doc,
    reason = "The panic is only possible with a corrupt DB"
)]
#[inline]
pub fn get_block_extended_header_from_height(
    block_height: &BlockHeight,
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
) -> DbResult<ExtendedBlockHeader> {
    let block_info_tape_reader = tapes.fixed_sized_tape_reader::<BlockInfo>(BLOCK_INFOS);

    let block_info = block_info_tape_reader
        .try_get(*block_height)
        .ok_or(RuntimeError::KeyNotFound)?;
    let miner_tx_info = tapes
        .fixed_sized_tape_reader::<TxInfo>(TX_INFOS)
        .try_get((block_info.mining_tx_index))
        .ok_or(RuntimeError::KeyNotFound)?;

    let reader = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let block_header_blob = reader
        .try_get_range(block_info.blob_idx..miner_tx_info.pruned_blob_idx)
        .unwrap();
    let block_header = BlockHeader::read(&mut block_header_blob.as_ref())?;

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
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
) -> DbResult<(ExtendedBlockHeader, BlockHeight)> {
    let height = chain_height(tables.block_heights())?.saturating_sub(1);
    let header = get_block_extended_header_from_height(&height, tables, tapes)?;
    Ok((header, height))
}

//---------------------------------------------------------------------------------------------------- Block
/// Retrieve a [`Block`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block(
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
    block_height: &BlockHeight,
) -> DbResult<Block> {
    let block_info_tape_reader = tapes.fixed_sized_tape_reader::<BlockInfo>(BLOCK_INFOS);

    let block_info = block_info_tape_reader
        .try_get(*block_height)
        .ok_or(RuntimeError::KeyNotFound)?;

    let block_blob_end_idx = {
        match tapes
            .fixed_sized_tape_reader::<TxInfo>(TX_INFOS)
            .try_get((block_info.mining_tx_index))
        {
            Some(tx_info) => {
                if tx_info.height != *block_height {
                    block_info_tape_reader
                        .try_get((*block_height + 1))
                        .ok_or(RuntimeError::KeyNotFound)?
                        .blob_idx
                } else {
                    tx_info.pruned_blob_idx
                }
            }
            None => tables.blob_tape_ends().get(&1)?.pruned_tape,
        }
    };

    let reader = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let block_blob = reader
        .try_get_range(block_info.blob_idx..block_blob_end_idx)
        .unwrap();

    Ok(Block::read(&mut block_blob.as_ref()).unwrap())
}

/// Retrieve a [`Block`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_by_hash(
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
    block_hash: &BlockHash,
) -> DbResult<Block> {
    let block_height = tables.block_heights().get(block_hash)?;
    get_block(tables, tapes, &block_height)
}

//---------------------------------------------------------------------------------------------------- Misc

/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(
    block_hash: &BlockHash,
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> DbResult<BlockHeight> {
    table_block_heights.get(block_hash)
}

/// Check if a block exists in the database.
///
/// # Errors
/// Note that this will never return `Err(RuntimeError::KeyNotFound)`,
/// as in that case, `Ok(false)` will be returned.
///
/// Other errors may still occur.
#[inline]
pub fn block_exists(
    block_hash: &BlockHash,
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> DbResult<bool> {
    table_block_heights.contains(block_hash)
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
