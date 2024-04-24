//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use bytemuck::TransparentWrapper;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, Scalar};
use monero_serai::{
    block::Block,
    transaction::{Input, Timelock, Transaction},
};

use cuprate_types::{ExtendedBlockHeader, TransactionVerificationData, VerifiedBlockInformation};

use crate::{
    database::{DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::{
        blockchain::{chain_height, cumulative_generated_coins},
        key_image::{add_key_image, remove_key_image},
        macros::doc_error,
        output::{
            add_output, add_rct_output, get_rct_num_outputs, remove_output, remove_rct_output,
        },
        tx::{add_tx, get_num_tx, remove_tx},
    },
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        AmountIndex, BlockHash, BlockHeight, BlockInfo, KeyImage, Output, OutputFlags,
        PreRctOutputId, RctOutput, TxHash,
    },
    StorableVec,
};

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
/// - `block.height` is not != [`chain_height`]
///
/// # Already exists
/// This function will operate normally even if `block` already
/// exists, i.e., this function will not return `Err` even if you
/// call this function infinitely with the same block.
// no inline, too big.
pub fn add_block(
    block: &VerifiedBlockInformation,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    //------------------------------------------------------ Check preconditions first

    // Cast height to `u32` for storage (handled at top of function).
    // Panic (should never happen) instead of allowing DB corruption.
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1560020991>
    let Ok(height) = u32::try_from(block.height) else {
        panic!("block.height ({}) > u32::MAX", block.height);
    };

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
        assert_eq!(block.block.txs.len(), block.txs.len());
        for (i, tx) in block.txs.iter().enumerate() {
            assert_eq!(tx.tx_blob, tx.tx.serialize());
            assert_eq!(tx.tx_hash, block.block.txs[i]);
        }
    }

    //------------------------------------------------------ Transaction / Outputs / Key Images
    for tx_verification_data in &block.txs {
        add_tx(tx_verification_data, &chain_height, tables)?;
    }

    //------------------------------------------------------ Block Info

    // INVARIANT: must be below the above transaction loop since this
    // RCT output count needs account for _this_ block's outputs.
    let cumulative_rct_outs = get_rct_num_outputs(tables.rct_outputs())?;

    let cumulative_generated_coins =
        cumulative_generated_coins(&block.height.saturating_sub(1), tables.block_infos())?
            + block.generated_coins;

    // Block Info.
    tables.block_infos_mut().put(
        &block.height,
        &BlockInfo {
            cumulative_generated_coins,
            cumulative_rct_outs,
            timestamp: block.block.header.timestamp,
            cumulative_difficulty: block.cumulative_difficulty,
            block_hash: block.block_hash,
            // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
            weight: block.weight as u64,
            long_term_weight: block.long_term_weight as u64,
        },
    )?;

    // Block blobs.
    tables
        .block_blobs_mut()
        .put(&block.height, StorableVec::wrap_ref(&block.block_blob))?;

    // Block heights.
    tables
        .block_heights_mut()
        .put(&block.block_hash, &block.height)?;

    Ok(())
}

//---------------------------------------------------------------------------------------------------- `pop_block`
/// Remove the top/latest block from the database.
///
/// The removed block's height and hash are returned.
#[doc = doc_error!()]
// no inline, too big
pub fn pop_block(
    tables: &mut impl TablesMut,
) -> Result<(BlockHeight, BlockHash, Block), RuntimeError> {
    //------------------------------------------------------ Block Info
    // Remove block data from tables.
    let (block_height, block_hash) = {
        let (block_height, block_info) = tables.block_infos_mut().pop_last()?;
        (block_height, block_info.block_hash)
    };

    // Block heights.
    tables.block_heights_mut().delete(&block_hash)?;

    // Block blobs.
    // We deserialize the block blob into a `Block`, such
    // that we can remove the associated transactions later.
    let block_blob = tables.block_blobs_mut().take(&block_height)?.0;
    let block = Block::read(&mut block_blob.as_slice())?;

    //------------------------------------------------------ Transaction / Outputs / Key Images
    for tx_hash in &block.txs {
        remove_tx(tx_hash, tables)?;
    }

    Ok((block_height, block_hash, block))
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
) -> Result<ExtendedBlockHeader, RuntimeError> {
    get_block_extended_header_from_height(&tables.block_heights().get(block_hash)?, tables)
}

/// Same as [`get_block_extended_header`] but with a [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header_from_height(
    block_height: &BlockHeight,
    tables: &impl Tables,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    let block_info = tables.block_infos().get(block_height)?;
    let block_blob = tables.block_blobs().get(block_height)?.0;
    let block = Block::read(&mut block_blob.as_slice())?;

    // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
    #[allow(clippy::cast_possible_truncation)]
    Ok(ExtendedBlockHeader {
        version: block.header.major_version,
        vote: block.header.minor_version,
        timestamp: block.header.timestamp,
        cumulative_difficulty: block_info.cumulative_difficulty,
        block_weight: block_info.weight as usize,
        long_term_weight: block_info.long_term_weight as usize,
    })
}

/// Return the top/latest [`ExtendedBlockHeader`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header_top(
    tables: &impl Tables,
) -> Result<(ExtendedBlockHeader, BlockHeight), RuntimeError> {
    let height = chain_height(tables.block_heights())?.saturating_sub(1);
    let header = get_block_extended_header_from_height(&height, tables)?;
    Ok((header, height))
}

//---------------------------------------------------------------------------------------------------- Misc
/// Retrieve a [`BlockInfo`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_info(
    block_height: &BlockHeight,
    table_block_infos: &impl DatabaseRo<BlockInfos>,
) -> Result<BlockInfo, RuntimeError> {
    table_block_infos.get(block_height)
}

/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(
    block_hash: &BlockHash,
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<BlockHeight, RuntimeError> {
    table_block_heights.get(block_hash)
}

/// Check if a block exists in the database.
#[doc = doc_error!()]
#[inline]
pub fn block_exists(
    block_hash: &BlockHash,
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<bool, RuntimeError> {
    table_block_heights.contains(block_hash)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[allow(
    clippy::significant_drop_tightening,
    clippy::cognitive_complexity,
    clippy::too_many_lines
)]
mod test {
    use hex_literal::hex;
    use pretty_assertions::assert_eq;

    use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx2, block_v9_tx3, tx_v2_rct3};

    use super::*;
    use crate::{
        ops::tx::{get_tx, tx_exists},
        tests::{assert_all_tables_are_empty, tmp_concrete_env, AssertTableLen},
        Env,
    };

    /// Tests all above block functions.
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    fn all_block_functions() {
        let (env, tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let mut blocks = [
            block_v1_tx2().clone(),
            block_v9_tx3().clone(),
            block_v16_tx0().clone(),
        ];
        // HACK: `add_block()` asserts blocks with non-sequential heights
        // cannot be added, to get around this, manually edit the block height.
        for (height, block) in blocks.iter_mut().enumerate() {
            block.height = height as u64;
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
                block_blobs: 3,
                block_heights: 3,
                key_images: 69,
                num_outputs: 38,
                pruned_tx_blobs: 0,
                prunable_hashes: 0,
                outputs: 107,
                prunable_tx_blobs: 0,
                rct_outputs: 6,
                tx_blobs: 5,
                tx_ids: 5,
                tx_heights: 5,
                tx_unlock_time: 0,
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
                assert_eq!(b1.version, b2.block.header.major_version);
                assert_eq!(b1.vote, b2.block.header.minor_version);
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
                    assert_eq!(tx.tx_hash, block.block.txs[i]);
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

                let (popped_height, popped_hash, popped_block) = pop_block(&mut tables).unwrap();

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
        let (env, tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        let mut block = block_v9_tx3().clone();

        block.height = u64::from(u32::MAX) + 1;
        add_block(&block, &mut tables).unwrap();
    }

    /// We should panic if: `block.height` != the chain height
    #[test]
    #[should_panic(
        expected = "assertion `left == right` failed: block.height (123) != chain_height (1)\n  left: 123\n right: 1"
    )]
    fn block_height_not_chain_height() {
        let (env, tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        let mut block = block_v9_tx3().clone();
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
