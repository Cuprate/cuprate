//! Block functions.

use std::sync::RwLock;
//---------------------------------------------------------------------------------------------------- Import
use bytemuck::TransparentWrapper;
use bytes::Bytes;
use monero_oxide::{
    block::{Block, BlockHeader},
    transaction::Transaction,
};

use cuprate_database::{
    DbResult, RuntimeError, StorableVec, {DatabaseRo, DatabaseRw},
};
use cuprate_helper::cast::usize_to_u64;
use cuprate_helper::{
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
    tx::tx_fee,
};
use cuprate_linear_tape::LinearTape;
use cuprate_types::{
    AltBlockInformation, BlockCompleteEntry, ChainId, ExtendedBlockHeader, HardFork,
    TransactionBlobs, VerifiedBlockInformation, VerifiedTransactionInformation,
};

use crate::{
    ops::{
        alt_block,
        blockchain::{chain_height, cumulative_generated_coins},
        macros::doc_error,
        output::get_rct_num_outputs,
        tx::{add_tx, remove_tx},
    },
    tables::{BlockHeights, BlockInfos, Tables, TablesIter, TablesMut},
    types::{BlockHash, BlockHeight, BlockInfo},
};
use crate::types::RctOutput;

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
pub fn add_block(block: &VerifiedBlockInformation, tables: &mut impl TablesMut, rct_tape: &RwLock<LinearTape<RctOutput>>) -> DbResult<()> {
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
            assert_eq!(tx.tx_blob, tx.tx.serialize());
            assert_eq!(tx.tx_hash, block.block.transactions[i]);
        }
    }

    //------------------------------------------------------ Transaction / Outputs / Key Images
    let mut rct_outputs = Vec::with_capacity(block.txs.len() * 2);
    let mut lock = rct_tape.write().unwrap();

    let mut numb_rct_outs = lock.len() as u64;
    // Add the miner transaction first.
    let mining_tx_index = {
        let tx = &block.block.miner_transaction;
        add_tx(tx, &tx.serialize(), &tx.hash(), &chain_height, &mut numb_rct_outs, tables, &mut rct_outputs)?
    };

    for tx in &block.txs {
        add_tx(&tx.tx, &tx.tx_blob, &tx.tx_hash, &chain_height, &mut numb_rct_outs, tables, &mut rct_outputs)?;
    }

    if !rct_outputs.is_empty() {
        let mut appender = lock.appender();
        if appender.push_entries(&rct_outputs).is_err() {
            appender.resize(20 * 1024 * 1024 * 1024).unwrap();
            appender.push_entries(&rct_outputs).unwrap()
        }
        appender.flush_async().unwrap();

        drop(appender);
    }
    //------------------------------------------------------ Block Info

    // INVARIANT: must be below the above transaction loop since this
    // RCT output count needs account for _this_ block's outputs.
    let cumulative_rct_outs = lock.len() as u64;

    // `saturating_add` is used here as cumulative generated coins overflows due to tail emission.
    let cumulative_generated_coins =
        cumulative_generated_coins(&block.height.saturating_sub(1), tables.block_infos())?
            .saturating_add(block.generated_coins);

    let (cumulative_difficulty_low, cumulative_difficulty_high) =
        split_u128_into_low_high_bits(block.cumulative_difficulty);

    // Block Info.
    tables.block_infos_mut().put(
        &block.height,
        &BlockInfo {
            cumulative_difficulty_low,
            cumulative_difficulty_high,
            cumulative_generated_coins,
            cumulative_rct_outs,
            timestamp: block.block.header.timestamp,
            block_hash: block.block_hash,
            weight: block.weight,
            long_term_weight: block.long_term_weight,
            mining_tx_index,
        },
    )?;

    // Block header blob.
    tables.block_header_blobs_mut().put(
        &block.height,
        StorableVec::wrap_ref(&block.block.header.serialize()),
    )?;

    // Block transaction hashes
    tables.block_txs_hashes_mut().put(
        &block.height,
        StorableVec::wrap_ref(&block.block.transactions),
    )?;

    // Block heights.
    tables
        .block_heights_mut()
        .put(&block.block_hash, &block.height)?;

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
    rct_tape: &RwLock<LinearTape<RctOutput>>,
) -> DbResult<(BlockHeight, BlockHash, Block)> {
    //------------------------------------------------------ Block Info
    // Remove block data from tables.
    let (block_height, block_info) = tables.block_infos_mut().pop_last()?;

    // Block heights.
    tables.block_heights_mut().delete(&block_info.block_hash)?;

    // Block blobs.
    //
    // We deserialize the block header blob and mining transaction blob
    // to form a `Block`, such that we can remove the associated transactions
    // later.
    let block_header = tables.block_header_blobs_mut().take(&block_height)?.0;
    let block_txs_hashes = tables.block_txs_hashes_mut().take(&block_height)?.0;
    let miner_transaction = tables.tx_blobs().get(&block_info.mining_tx_index)?.0;
    let block = Block {
        header: BlockHeader::read(&mut block_header.as_slice())?,
        miner_transaction: Transaction::read(&mut miner_transaction.as_slice())?,
        transactions: block_txs_hashes,
    };

    //------------------------------------------------------ Transaction / Outputs / Key Images
    let mut rct_outputs = 0;
    remove_tx(&block.miner_transaction.hash(), &mut rct_outputs, tables)?;

    let remove_tx_iter = block.transactions.iter().map(|tx_hash| {
        let (_, tx) = remove_tx(tx_hash, &mut rct_outputs, tables)?;
        Ok::<_, RuntimeError>(tx)
    });
    

    if let Some(chain_id) = move_to_alt_chain {
        let txs = remove_tx_iter
            .map(|result| {
                let tx = result?;
                Ok(VerifiedTransactionInformation {
                    tx_weight: tx.weight(),
                    tx_blob: tx.serialize(),
                    tx_hash: tx.hash(),
                    fee: tx_fee(&tx),
                    tx,
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

    let mut lock = rct_tape.write().unwrap();
    let mut popper = lock.popper();
    popper.pop_entries(rct_outputs as usize);
    popper.flush()?;

    Ok((block_height, block_info.block_hash, block))
}

//---------------------------------------------------------------------------------------------------- `get_block_blob_with_tx_indexes`
/// Retrieve a block's raw bytes, the index of the miner transaction and the number of non miner-txs in the block.
///
#[doc = doc_error!()]
pub fn get_block_blob_with_tx_indexes(
    block_height: &BlockHeight,
    tables: &impl Tables,
) -> Result<(Vec<u8>, u64, usize), RuntimeError> {
    let miner_tx_idx = tables.block_infos().get(block_height)?.mining_tx_index;

    let block_txs = tables.block_txs_hashes().get(block_height)?.0;
    let numb_txs = block_txs.len();

    // Get the block header
    let mut block = tables.block_header_blobs().get(block_height)?.0;

    // Add the miner tx to the blob.
    let mut miner_tx_blob = tables.tx_blobs().get(&miner_tx_idx)?.0;
    block.append(&mut miner_tx_blob);

    // Add the blocks tx hashes.
    monero_oxide::io::write_varint(&block_txs.len(), &mut block)
        .expect("The number of txs per block will not exceed u64::MAX");

    let block_txs_bytes = bytemuck::must_cast_slice(&block_txs);
    block.extend_from_slice(block_txs_bytes);

    Ok((block, miner_tx_idx, numb_txs))
}

//---------------------------------------------------------------------------------------------------- `get_block_complete_entry_*`
/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry(
    block_hash: &BlockHash,
    tables: &impl TablesIter,
) -> Result<BlockCompleteEntry, RuntimeError> {
    let block_height = tables.block_heights().get(block_hash)?;
    get_block_complete_entry_from_height(&block_height, tables)
}

/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry_from_height(
    block_height: &BlockHeight,
    tables: &impl TablesIter,
) -> Result<BlockCompleteEntry, RuntimeError> {
    let (block_blob, miner_tx_idx, numb_non_miner_txs) =
        get_block_blob_with_tx_indexes(block_height, tables)?;

    let first_tx_idx = miner_tx_idx + 1;

    let tx_blobs = (first_tx_idx..(usize_to_u64(numb_non_miner_txs) + first_tx_idx))
        .map(|idx| {
            let tx_blob = tables.tx_blobs().get(&idx)?.0;

            Ok(Bytes::from(tx_blob))
        })
        .collect::<Result<_, RuntimeError>>()?;

    Ok(BlockCompleteEntry {
        block: Bytes::from(block_blob),
        txs: TransactionBlobs::Normal(tx_blobs),
        pruned: false,
        block_weight: 0,
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
) -> DbResult<ExtendedBlockHeader> {
    get_block_extended_header_from_height(&tables.block_heights().get(block_hash)?, tables)
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
) -> DbResult<ExtendedBlockHeader> {
    let block_info = tables.block_infos().get(block_height)?;
    let block_header_blob = tables.block_header_blobs().get(block_height)?.0;
    let block_header = BlockHeader::read(&mut block_header_blob.as_slice())?;

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
) -> DbResult<(ExtendedBlockHeader, BlockHeight)> {
    let height = chain_height(tables.block_heights())?.saturating_sub(1);
    let header = get_block_extended_header_from_height(&height, tables)?;
    Ok((header, height))
}

//---------------------------------------------------------------------------------------------------- Block
/// Retrieve a [`Block`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block(tables: &impl Tables, block_height: &BlockHeight) -> DbResult<Block> {
    let header_blob = tables.block_header_blobs().get(block_height)?.0;
    let header = BlockHeader::read(&mut header_blob.as_slice())?;

    let transactions = tables.block_txs_hashes().get(block_height)?.0;
    let miner_tx_id = tables.block_infos().get(block_height)?.mining_tx_index;
    let miner_transaction = crate::ops::tx::get_tx_from_id(&miner_tx_id, tables.tx_blobs())?;

    Ok(Block {
        header,
        miner_transaction,
        transactions,
    })
}

/// Retrieve a [`Block`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_by_hash(tables: &impl Tables, block_hash: &BlockHash) -> DbResult<Block> {
    let block_height = tables.block_heights().get(block_hash)?;
    get_block(tables, &block_height)
}

//---------------------------------------------------------------------------------------------------- Misc
/// Retrieve a [`BlockInfo`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_info(
    block_height: &BlockHeight,
    table_block_infos: &impl DatabaseRo<BlockInfos>,
) -> DbResult<BlockInfo> {
    table_block_infos.get(block_height)
}

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
