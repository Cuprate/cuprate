//! Blocks.

use std::sync::Arc;

//---------------------------------------------------------------------------------------------------- Import
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

use super::{output::get_rct_num_outputs, tx::get_num_tx};

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
                    add_key_image(tables.key_images_mut(), key_image.compress().as_bytes())?;
                }
                // This is a miner transaction, set it for later use.
                Input::Gen(_) => miner_tx = true,
            }
        }

        // Remove each output in the transaction.
        for output in tx.prefix.outputs {
            // Outputs with clear amounts.
            if let Some(amount) = output.amount {
                // RingCT miner outputs.
                if miner_tx && tx.prefix.version == 2 {
                    let amount_index = get_rct_num_outputs(tables.rct_outputs_mut())?;
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
                let amount_index = get_rct_num_outputs(tables.rct_outputs_mut())?;
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
#[allow(unused_assignments)] // `block_weight` gets returned at the end?
pub fn get_block(
    tables: &impl Tables,
    block_hash: BlockHash,
) -> Result<VerifiedBlockInformation, RuntimeError> {
    let height = tables.block_heights().get(&block_hash)?;
    let block_info = tables.block_infos().get(&height)?;
    let block_blob = tables.block_blobs().get(&height)?.0;
    let block = Block::read(&mut block_blob.as_slice())?;

    let mut txs = Vec::with_capacity(block.txs.len());
    let mut block_weight = 0;

    // Transactions.
    for tx_blob in &block.txs {
        let tx = Transaction::read(&mut tx_blob.as_slice())?;

        let tx_weight = tx.weight();
        block_weight += tx_weight;

        txs.push(Arc::new(TransactionVerificationData {
            tx_weight,
            tx_blob: tx_blob.to_vec(),
            tx_hash: tx.hash(),
            fee: todo!(), // TODO: how to calculate?
            tx,
        }));
    }

    // Sum the amount of generated coins for this block.
    let generated_coins = block
        .miner_tx
        .prefix
        .inputs
        .iter()
        .map(|input| match input {
            Input::Gen(amount) => *amount,
            Input::ToKey { .. } => 0,
        })
        .sum();

    Ok(VerifiedBlockInformation {
        block,
        txs,
        block_hash,
        pow_hash: todo!(),
        height,
        generated_coins,
        weight: block_weight,
        long_term_weight: todo!(),
        cumulative_difficulty: todo!(),
        block_blob,
    })
}

/// Same as [`get_block`] but with a [`BlockHeight`].
///
/// Note: This is more expensive than the above.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block_from_height(
    tables: &impl Tables,
    block_height: BlockHeight,
) -> Result<VerifiedBlockInformation, RuntimeError> {
    get_block(tables, tables.block_infos().get(&block_height)?.block_hash)
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
    block_hash: BlockHash,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    let height = tables.block_heights().get(&block_hash)?;
    let block_info = tables.block_infos().get(&height)?;
    let block_blob = tables.block_blobs().get(&height)?.0;
    let block = Block::read(&mut block_blob.as_slice())?;

    Ok(ExtendedBlockHeader {
        version: block.header.major_version,
        vote: block.header.minor_version,
        timestamp: block.header.timestamp,
        cumulative_difficulty: todo!(),
        block_weight: todo!(),
        long_term_weight: todo!(),
    })
}

/// Same as [`get_block_header`] but with a [`BlockHeight`].
///
/// Note: This is more expensive than the above.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block_header_from_height(
    tables: &impl Tables,
    block_height: BlockHeight,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    get_block_header(tables, tables.block_infos().get(&block_height)?.block_hash)
}

//---------------------------------------------------------------------------------------------------- `get_block_top_*`
/// Return the top/latest block from the database.
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_block_top(tables: &impl Tables) -> Result<VerifiedBlockInformation, RuntimeError> {
    get_block_from_height(
        tables,
        crate::ops::blockchain::chain_height(tables.block_heights())?.saturating_sub(1),
    )
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
