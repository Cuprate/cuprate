//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::{Timelock, Transaction};

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
        BlockBlobs, BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s, KeyImages, NumOutputs,
        Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut,
        TxHeights, TxIds, TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        AmountIndex, BlockHash, BlockHeight, BlockInfoLatest, BlockInfoV1, BlockInfoV2,
        BlockInfoV3, KeyImage, Output, PreRctOutputId, RctOutput, TxHash,
    },
};

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
    block: &VerifiedBlockInformation,
) -> Result<(), RuntimeError> {
    // Block Info.
    //
    // Branch on the hard fork version (`major_version`)
    // and add the block to the appropriate table.
    // <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    //
    // FIXME: use `match` with ranges when stable:
    // <https://github.com/rust-lang/rust/issues/37854>
    if block.block.header.major_version < 4 {
        tables.block_info_v1s_mut().put(
            &block.height,
            &BlockInfoV1 {
                timestamp: block.block.header.timestamp,
                total_generated_coins: block.generated_coins,
                weight: block.weight as u64, // TODO
                #[allow(clippy::cast_possible_truncation)] // TODO
                cumulative_difficulty: block.cumulative_difficulty as u64, // TODO
                block_hash: block.block_hash,
            },
        )
    } else if block.block.header.major_version < 10 {
        tables.block_info_v2s_mut().put(
            &block.height,
            #[allow(clippy::cast_possible_truncation)] // TODO
            &BlockInfoV2 {
                timestamp: block.block.header.timestamp,
                total_generated_coins: block.generated_coins,
                weight: block.weight as u64, // TODO
                #[allow(clippy::cast_possible_truncation)] // TODO
                cumulative_difficulty: block.cumulative_difficulty as u64, // TODO
                block_hash: block.block_hash,
                cumulative_rct_outs: todo!(), // TODO
            },
        )
    } else {
        tables.block_info_v3s_mut().put(
            &block.height,
            &BlockInfoV3 {
                timestamp: block.block.header.timestamp,
                total_generated_coins: block.generated_coins,
                weight: block.weight as u64, // TODO
                cumulative_difficulty: block.cumulative_difficulty,
                block_hash: block.block_hash,
                cumulative_rct_outs: todo!(),                    // TODO
                long_term_weight: block.long_term_weight as u64, // TODO
            },
        )
    }?;

    // Block blobs.
    // TODO: what is a block blob in Cuprate's case?
    tables.block_blobs_mut().put(&block.height, todo!())?;

    // Block heights.
    tables
        .block_heights_mut()
        .put(&block.block_hash, &block.height)?;

    // Transaction & Outputs.
    for tx in block.txs {
        let tx: &Transaction = &tx.tx;

        add_tx(
            tx,
            tables.block_heights_mut(),
            tables.tx_ids_mut(),
            tables.tx_heights_mut(),
            tables.tx_unlock_time_mut(),
            tables.prunable_hashes_mut(),
            tables.prunable_tx_blobs_mut(),
        )?;

        // Output data.
        for output in tx.prefix.outputs {
            // Key images.
            add_key_image(tables.key_images_mut(), output.key.as_bytes())?;

            // Pre-RingCT outputs.
            if let Some(amount) = output.amount {
                add_output(
                    amount,
                    &Output {
                        key: *output.key.as_bytes(),
                        height: todo!(),
                        output_flags: todo!(),
                        tx_idx: todo!(),
                    },
                    tables.outputs_mut(),
                    tables.num_outputs_mut(),
                )?;
            // RingCT outputs.
            } else {
                add_rct_output(
                    &RctOutput {
                        key: todo!(),
                        height: todo!(),
                        output_flags: todo!(),
                        tx_idx: todo!(),
                        commitment: todo!(),
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
    let (block_height, block_hash) = if todo!() {
        let (block_height, block_info) = tables.block_info_v1s_mut().pop_last()?;
        (block_height, block_info.block_hash)
    } else if todo!() {
        let (block_height, block_info) = tables.block_info_v2s_mut().pop_last()?;
        (block_height, block_info.block_hash)
    } else {
        let (block_height, block_info) = tables.block_info_v3s_mut().pop_last()?;
        (block_height, block_info.block_hash)
    };

    // Block blobs.
    tables.block_blobs_mut().delete(&block_height)?;

    // Block heights.
    tables.block_heights_mut().delete(&block_hash)?;

    // Transaction & Outputs.
    for () in /* block.txs */ std::iter::empty::<()>() {
        let tx: &Transaction = todo!();
        let tx_hash: &TxHash = todo!();

        remove_tx(
            tx_hash,
            tables.tx_ids_mut(),
            tables.tx_heights_mut(),
            tables.tx_unlock_time_mut(),
            tables.prunable_hashes_mut(),
            tables.prunable_tx_blobs_mut(),
        )?;

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
                    tables.outputs_mut(),
                    tables.num_outputs_mut(),
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
    let top_block_height = crate::ops::blockchain::height(tables.block_heights())?;
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
