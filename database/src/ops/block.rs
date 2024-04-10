//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, Scalar};
use monero_serai::{
    block::Block,
    transaction::{Input, Timelock, Transaction},
};

use cuprate_types::{ExtendedBlockHeader, VerifiedBlockInformation};

use crate::{
    database::{DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::{
        key_image::{add_key_image, remove_key_image},
        macros::doc_error,
        output::{add_output, add_rct_output, remove_output, remove_rct_output},
        tx::{add_tx, remove_tx},
    },
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        AmountIndex, BlockHash, BlockHeight, BlockInfo, KeyImage, Output, PreRctOutputId,
        RctOutput, TxHash,
    },
    StorableVec,
};

use super::tx::get_num_tx;

//---------------------------------------------------------------------------------------------------- `add_block_*`
/// Add a [`VerifiedBlockInformation`] to the database.
///
/// This extracts all the data from the input block and
/// maps and adds them to the appropriate database tables.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[allow(clippy::too_many_lines)]
// no inline, too big.
pub fn add_block(
    tables: &mut impl TablesMut,
    block: VerifiedBlockInformation,
) -> Result<(), RuntimeError> {
    let VerifiedBlockInformation {
        block,
        txs,
        block_hash,
        pow_hash,
        height,
        generated_coins,
        weight,
        long_term_weight,
        cumulative_difficulty,
        block_blob,
    } = block;

    let cumulative_rct_outs = crate::ops::output::get_rct_num_outputs(tables.rct_outputs_mut())?;

    // Block Info.
    tables.block_infos_mut().put(
        &height,
        &BlockInfo {
            timestamp: block.header.timestamp,
            total_generated_coins: generated_coins,
            cumulative_difficulty,
            block_hash,
            cumulative_rct_outs,
            // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
            weight: weight as u64,
            long_term_weight: long_term_weight as u64,
        },
    )?;

    // Block blobs.
    tables
        .block_blobs_mut()
        .put(&height, &StorableVec(block_blob))?;

    // Block heights.
    tables.block_heights_mut().put(&block_hash, &height)?;

    // Transaction & Outputs.
    for tx in txs {
        let tx: &Transaction = &tx.tx;
        add_tx(tx, tables)?;

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
                    add_key_image(tables.key_images_mut(), key_image.compress().as_bytes())?;
                }
                // This is a miner transaction, set it for later use.
                Input::Gen(_) => miner_tx = true,
            }
        }

        // Output bit flags.
        // Set to a non-zero bit value if the unlock time is non-zero.
        // TODO: use bitflags.
        let output_flags = match tx.prefix.timelock {
            Timelock::None => 0b0000_0000,
            Timelock::Block(_) | Timelock::Time(_) => 0b0000_0001,
        };

        // TODO: Output types have `height: u32` but ours is u64 - is this cast ok?
        #[allow(clippy::cast_possible_truncation)]
        let height = height as u32;

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
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn pop_block(tables: &mut impl TablesMut) -> Result<BlockHeight, RuntimeError> {
    // Remove block data from tables.
    //
    // TODO: What table to pop from here?
    // Start with v3, if thats empty, try v2, etc?
    //
    // Branch depending on `height` and known hard fork points?
    let (block_height, block_hash) = {
        let (block_height, block_info) = tables.block_infos_mut().pop_last()?;
        (block_height, block_info.block_hash)
    };

    // Block blobs.
    // We deserialize the block blob into a `Block`, such
    // that we can remove the associated transactions later.
    let block_blob = tables.block_blobs_mut().take(&block_height)?.0;
    let block = Block::read(&mut block_blob.as_slice())?;

    // Block heights.
    tables.block_heights_mut().delete(&block_hash)?;

    // Transaction & Outputs.
    for tx in block.txs {
        let tx: &Transaction = todo!();
        let tx_hash: &TxHash = todo!();

        remove_tx(tx_hash, tables)?;

        // Output data.
        for output in tx.prefix.outputs {
            // Key images.
            remove_key_image(tables.key_images_mut(), output.key.as_bytes())?;

            let amount_index: AmountIndex = todo!();

            // Pre-RingCT outputs.
            if let Some(amount) = output.amount {
                remove_output(
                    &PreRctOutputId {
                        amount,
                        amount_index,
                    },
                    tables,
                )?;
            // RingCT outputs.
            } else {
                remove_rct_output(&amount_index, tables.rct_outputs_mut())?;
            }
        }
    }

    Ok(block_height)
}

//---------------------------------------------------------------------------------------------------- `get_block_*`
/// Retrieve a [`VerifiedBlockInformation`] from the database.
///
/// This extracts all the data from the database tables
/// needed to create a full `VerifiedBlockInformation`.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block(
    tables: &impl Tables,
    height: BlockHeight,
) -> Result<VerifiedBlockInformation, RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_block_header_*`
/// Retrieve a [`ExtendedBlockHeader`] from the database.
///
/// This extracts all the data from the database tables
/// needed to create a full `ExtendedBlockHeader`.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block_header(
    tables: &impl Tables,
    height: BlockHeight,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_block_top_*`
/// Return the top block from the database.
///
/// This is the same as [`pop_block()`], but it does
/// not remove the block, it only retrieves it.
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block_top(tables: &impl Tables) -> Result<VerifiedBlockInformation, RuntimeError> {
    let top_block_height = crate::ops::blockchain::chain_height(tables.block_heights())?;
    get_block(tables, top_block_height)
}

//---------------------------------------------------------------------------------------------------- `get_block_height_*`
/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(
    table_block_heights: &impl DatabaseRo<BlockHeights>,
    block_hash: &BlockHash,
) -> Result<BlockHeight, RuntimeError> {
    table_block_heights.get(block_hash)
}

//---------------------------------------------------------------------------------------------------- Misc
/// Check if a block exists in the database.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn block_exists(
    table_block_heights: &impl DatabaseRo<BlockHeights>,
    block_hash: &BlockHash,
) -> Result<bool, RuntimeError> {
    table_block_heights.contains(block_hash)
}
