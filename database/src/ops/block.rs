//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

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
        blockchain::chain_height,
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
/// maps and adds them to the appropriate database tables.
///
#[doc = doc_error!()]
///
/// # Panics
/// This function will panic if `block.height > u32::MAX` (not normally possible).
#[allow(clippy::too_many_lines)]
// no inline, too big.
pub fn add_block(
    block: &VerifiedBlockInformation,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    let block_height = chain_height(tables.block_heights())?;
    if block.height != block_height {
        // TODO: what to do when the caller:
        // - provided a block that isn't the chain tip
        // - provided a block that already exists
    }

    let cumulative_rct_outs = get_rct_num_outputs(tables.rct_outputs())?;

    // Block Info.
    tables.block_infos_mut().put(
        &block.height,
        &BlockInfo {
            timestamp: block.block.header.timestamp,
            total_generated_coins: block.generated_coins,
            cumulative_difficulty: block.cumulative_difficulty,
            block_hash: block.block_hash,
            cumulative_rct_outs,
            // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
            weight: block.weight as u64,
            long_term_weight: block.long_term_weight as u64,
        },
    )?;

    // Block blobs.
    tables.block_blobs_mut().put(
        &block.height,
        bytemuck::TransparentWrapper::wrap_ref(&block.block_blob),
    )?;

    // Block heights.
    tables
        .block_heights_mut()
        .put(&block.block_hash, &block.height)?;

    // Cast height to `u32` for storage.
    // Panic (should never happen) instead of allowing DB corruption.
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1560020991>
    let height = u32::try_from(block.height).expect("height was > u32::MAX");

    // Transaction & Outputs.
    for tx in &block.txs {
        add_tx(tx, tables)?;
        let tx: &Transaction = &tx.tx;

        // ^
        // Everything above is adding tx data.
        // Everything below is for adding input/output data.
        // v

        // Is this a miner transaction?
        // Which table we add the output data to depends on this.
        // <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/blockchain_db/blockchain_db.cpp#L212-L216>
        let mut miner_tx = false;

        for inputs in &tx.prefix.inputs {
            match inputs {
                // Key images.
                Input::ToKey { key_image, .. } => {
                    add_key_image(key_image.compress().as_bytes(), tables.key_images_mut())?;
                }
                // This is a miner transaction, set it for later use.
                Input::Gen(_) => miner_tx = true,
            }
        }

        // Output bit flags.
        // Set to a non-zero bit value if the unlock time is non-zero.
        // TODO: use bitflags.
        let output_flags = match tx.prefix.timelock {
            Timelock::None => OutputFlags::NONE,
            Timelock::Block(_) | Timelock::Time(_) => OutputFlags::NON_ZERO_UNLOCK_TIME,
        };

        // Output data.
        for (i, output) in tx.prefix.outputs.iter().enumerate() {
            let tx_idx = get_num_tx(tables.tx_ids_mut())?;
            let key = *output.key.as_bytes();

            // Outputs with clear amounts.
            if let Some(amount) = output.amount {
                // RingCT (v2 transaction) miner outputs.
                if miner_tx && tx.prefix.version == 2 {
                    // Create commitment.
                    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1559489302>
                    // FIXME: implement lookup table for common values:
                    // <https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/ringct/rctOps.cpp#L322>
                    let commitment = (ED25519_BASEPOINT_POINT
                        + monero_serai::H() * Scalar::from(amount))
                    .compress()
                    .to_bytes();

                    add_rct_output(
                        &RctOutput {
                            key,
                            height,
                            output_flags,
                            tx_idx,
                            commitment,
                        },
                        tables.rct_outputs_mut(),
                    )?;
                // Pre-RingCT outputs.
                } else {
                    add_output(
                        amount,
                        &Output {
                            key,
                            height,
                            output_flags,
                            tx_idx,
                        },
                        tables,
                    )?;
                }
            // RingCT outputs.
            } else {
                let commitment = tx.rct_signatures.base.commitments[i].compress().to_bytes();
                add_rct_output(
                    &RctOutput {
                        key,
                        height,
                        output_flags,
                        tx_idx,
                        commitment,
                    },
                    tables.rct_outputs_mut(),
                )?;
            }
        }
    }

    Ok(())
}

//---------------------------------------------------------------------------------------------------- `pop_block_*`
/// Remove the top/latest block from the database.
#[doc = doc_error!()]
#[inline]
#[allow(clippy::missing_panics_doc)] // TODO: remove
#[allow(clippy::never_loop)] // TODO: remove me
#[allow(unused_assignments)]
pub fn pop_block(tables: &mut impl TablesMut) -> Result<(BlockHeight, BlockHash), RuntimeError> {
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

    // Transaction & Outputs.
    for tx_hash in block.txs {
        let (tx_id, tx_blob) = remove_tx(&tx_hash, tables)?;
        let tx = Transaction::read(&mut tx_blob.0.as_slice())?;

        // Is this a miner transaction?
        let mut miner_tx = false;
        for inputs in &tx.prefix.inputs {
            match inputs {
                // Key images.
                Input::ToKey { key_image, .. } => {
                    let result =
                        remove_key_image(key_image.compress().as_bytes(), tables.key_images_mut());

                    // TODO: all test tx's have the same key image so the 1st
                    // removal removes everything - fix me after we have real data.
                    if cfg!(test) {
                        match result {
                            Ok(()) | Err(RuntimeError::KeyNotFound) => (),
                            Err(e) => return Err(e),
                        }
                    } else {
                        result?;
                    }
                }
                // This is a miner transaction, set it for later use.
                Input::Gen(_) => miner_tx = true,
            }
        }

        // Remove each output in the transaction.
        for (i, output) in tx.prefix.outputs.into_iter().enumerate() {
            // Outputs with clear amounts.
            if let Some(amount) = output.amount {
                // RingCT miner outputs.
                if miner_tx && tx.prefix.version == 2 {
                    let amount_index = get_rct_num_outputs(tables.rct_outputs())?.saturating_sub(1);
                    remove_rct_output(&amount_index, tables.rct_outputs_mut())?;
                // Pre-RingCT outputs.
                } else {
                    let amount_index = tables.num_outputs_mut().take(&amount)?;
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
                let amount_index = get_rct_num_outputs(tables.rct_outputs())?.saturating_sub(1);
                remove_rct_output(&amount_index, tables.rct_outputs_mut())?;
            }
        }
    }

    Ok((block_height, block_hash))
}

//---------------------------------------------------------------------------------------------------- `get_block_extended_header_*`
/// Retrieve a [`ExtendedBlockHeader`] from the database.
///
/// This extracts all the data from the database tables
/// needed to create a full `ExtendedBlockHeader`.
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header(
    block_hash: BlockHash,
    tables: &impl Tables,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    let height = tables.block_heights().get(&block_hash)?;
    let block_info = tables.block_infos().get(&height)?;
    let block_blob = tables.block_blobs().get(&height)?.0;
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

/// Same as [`get_block_extended_header`] but with a [`BlockHeight`].
///
/// Note: This is more expensive than the above.
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header_from_height(
    block_height: &BlockHeight,
    tables: &impl Tables,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    get_block_extended_header(tables.block_infos().get(block_height)?.block_hash, tables)
}

/// Return the top/latest [`ExtendedBlockHeader`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header_top(
    tables: &impl Tables,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    get_block_extended_header_from_height(
        &chain_height(tables.block_heights())?.saturating_sub(1),
        tables,
    )
}

//---------------------------------------------------------------------------------------------------- `get_block_height_*`
/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(
    block_hash: &BlockHash,
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<BlockHeight, RuntimeError> {
    table_block_heights.get(block_hash)
}

//---------------------------------------------------------------------------------------------------- Misc
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
#[allow(clippy::significant_drop_tightening)]
mod test {
    use hex_literal::hex;
    use pretty_assertions::assert_eq;

    use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx513, block_v9_tx3, tx_v2_rct3};

    use super::*;
    use crate::{
        ops::tx::{get_tx, tx_exists},
        tests::{assert_all_tables_are_empty, tmp_concrete_env},
        Env,
    };

    /// TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO
    /// TODO: `cuprate_test_utils::data` should return `VerifiedBlockInformation`.
    /// The tests below should be testing _real_ blocks that has _real_ transactions/outputs/hashes/etc.
    ///
    /// As `VerifiedBlockInformation` contains some fields that
    /// we cannot actually produce in `cuprate_database`, the testing
    /// below will use not real but "close-enough" values.
    ///
    /// For example, a real `pow_hash` is not computable here without
    /// importing `PoW` code, so instead we fill it with dummy values.
    fn dummy_verified_block_information() -> VerifiedBlockInformation {
        let block = block_v9_tx3();

        // `pop_block()` finds and removes a block's transactions by its `block.txs` field
        // so we need to provide transactions that have the same hashes as `block_v9_tx3()`'s block.
        // The other contents are not real (fee, weight, etc).
        let tx = tx_v2_rct3();
        let mut txs = vec![];
        for tx_hash in [
            hex_literal::hex!("e2d39395dd1625b2d707b98af789e7eab9d24c2bd2978ec38ef910961a8cdcee"),
            hex_literal::hex!("e57440ec66d2f3b2a5fa2081af40128868973e7c021bb3877290db3066317474"),
            hex_literal::hex!("b6b4394d4ec5f08ad63267c07962550064caa8d225dd9ad6d739ebf60291c169"),
        ] {
            txs.push(Arc::new(TransactionVerificationData {
                tx_hash,
                tx: tx.clone(),
                tx_blob: tx.serialize(),
                tx_weight: tx.weight(),
                fee: 1_401_270_000,
            }));
        }

        VerifiedBlockInformation {
            block_hash: block.hash(),
            block_blob: block.serialize(),
            block,
            txs,                      // dummy
            pow_hash: [3; 32],        // dummy
            height: 3,                // dummy
            generated_coins: 3,       // dummy
            weight: 3,                // dummy
            long_term_weight: 3,      // dummy
            cumulative_difficulty: 3, // dummy
        }
    }

    /// Tests all above block functions.
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn all_block_functions() {
        let (env, tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let blocks: Vec<VerifiedBlockInformation> = vec![dummy_verified_block_information()];

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

            // TODO: fix this when new and _real_ blocks are added.
            // Assert only the proper tables were added to.
            assert_eq!(tables.block_infos().len().unwrap(), 1);
            assert_eq!(tables.block_blobs().len().unwrap(), 1);
            assert_eq!(tables.block_heights().len().unwrap(), 1);
            assert_eq!(tables.key_images().len().unwrap(), 2);
            assert_eq!(tables.num_outputs().len().unwrap(), 0);
            assert_eq!(tables.pruned_tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.prunable_hashes().len().unwrap(), 0);
            assert_eq!(tables.outputs().len().unwrap(), 0);
            assert_eq!(tables.prunable_tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.rct_outputs().len().unwrap(), 6);
            assert_eq!(tables.tx_blobs().len().unwrap(), 3);
            assert_eq!(tables.tx_ids().len().unwrap(), 3);
            assert_eq!(tables.tx_heights().len().unwrap(), 3);
            assert_eq!(tables.tx_unlock_time().len().unwrap(), 0);

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
                    get_block_extended_header(block.block_hash, &tables).unwrap();

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
                    // assert_eq!(tx.tx_hash, tx2.hash()); // TODO: we're using fake hashes for now, fix this.

                    // TODO: Assert output reads are OK.
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

                let (popped_height, popped_hash) = pop_block(&mut tables).unwrap();

                assert_eq!(block_hash, popped_hash);

                assert!(matches!(
                    get_block_extended_header(block_hash, &tables),
                    Err(RuntimeError::KeyNotFound)
                ));
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }
}
