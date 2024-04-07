//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::Timelock;

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
        BlockHash, BlockHeight, BlockInfoLatest, BlockInfoV1, BlockInfoV2, BlockInfoV3, KeyImage,
        Output, PreRctOutputId, RctOutput,
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

    // BlockBlobs: BlockHeight => BlockBlob
    // TODO: what is a block blob in Cuprate's case?
    tables.block_blobs_mut().put(&block.height, todo!())?;

    // BlockHeights: BlockHash => BlockHeight
    tables
        .block_heights_mut()
        .put(&block.block_hash, &block.height)?;

    // Transaction data.
    //
    // - NumOutputs:      Amount         => AmountIndex
    // - PrunedTxBlobs:   TxId           => PrunableBlob
    // - PrunableHashes:  TxId           => PrunableHash
    // - Outputs:         PreRctOutputId => Output
    // - PrunableTxBlobs: TxId           => PrunableBlob
    // - RctOutputs:      AmountIndex    => RctOutput
    // - TxIds:           TxHash         => TxId
    // - KeyImages:       KeyImage       => ()
    {
        for tx in block.txs {
            let tx_id = todo!();
            let prunable_blob = todo!();
            let prunable_hash = todo!();

            tables.pruned_tx_blobs_mut().put(&tx_id, prunable_blob)?;
            tables.prunable_hashes_mut().put(&tx_id, prunable_hash)?;

            for output in tx.tx.prefix.outputs {
                let amount = todo!();
                let amount_index = todo!();

                let pre_rct_output_id = PreRctOutputId {
                    amount,
                    amount_index,
                };

                add_key_image(tables.key_images_mut(), output.key.as_bytes())?;

                // RingCT outputs.
                if todo!() {
                    add_rct_output(
                        amount,
                        amount_index,
                        &RctOutput {
                            key: todo!(),
                            height: todo!(),
                            output_flags: todo!(),
                            tx_idx: todo!(),
                            commitment: todo!(),
                        },
                        tables.key_images_mut(),
                        tables.num_outputs_mut(),
                        tables.rct_outputs_mut(),
                    )?;
                // Pre-RingCT outputs.
                } else {
                    add_output(
                        amount,
                        amount_index,
                        &Output {
                            key: *output.key.as_bytes(),
                            height: todo!(),
                            output_flags: todo!(),
                            tx_idx: todo!(),
                        },
                        tables.key_images_mut(),
                        tables.num_outputs_mut(),
                        tables.outputs_mut(),
                    )?;
                }

                tables.tx_ids_mut().put(&tx.tx_hash, &tx_id)?;
                tables.tx_heights_mut().put(&tx_id, &block.height)?;

                let unlock_time = match tx.tx.prefix.timelock {
                    Timelock::None => todo!(),
                    Timelock::Block(height) => todo!(), // Calculate from height?
                    Timelock::Time(time) => time,
                };
                tables.tx_unlock_time_mut().put(&tx_id, &unlock_time)?;
            }
        }
    }

    Ok(())
}

//---------------------------------------------------------------------------------------------------- `pop_block_*`
/// Remove and return the top block from the database.
///
/// This pops the latest block from the database, and
/// constructs the data into the returned [`VerifiedBlockInformation`].
///
/// Consider using [`pop_block_cheap()`] if the returned block is unneeded.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
#[allow(clippy::missing_panics_doc)] // this should not panic
pub fn pop_block(tables: &mut impl TablesMut) -> Result<VerifiedBlockInformation, RuntimeError> {
    Ok(pop_block_inner::<true>(tables)?.expect("this should always return `Some`"))
}

/// A cheaper to call [`pop_block()`].
///
/// This is the same as `pop_block()` however it will
/// not construct and return the block removed, thus,
/// it should be faster to call in situations where the
/// returned block would not be used anyway.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn pop_block_cheap(tables: &mut impl TablesMut) -> Result<(), RuntimeError> {
    let option = pop_block_inner::<false>(tables)?;
    debug_assert!(option.is_none());
    Ok(())
}

/// Internal function that is used by:
/// - [`pop_block()`]
/// - [`pop_block_cheap()`]
///
/// The logic for "popping" the block is defined here,
/// although the `const RETURN: bool` will dictate if this function
/// constructs and returns the block wrapped in `Some` or `None`.
///
/// # Invariant
/// - `RETURN == true` -> This must return `Some`
/// - `RETURN == false` -> This must return `None`
#[inline]
fn pop_block_inner<const RETURN: bool>(
    tables: &mut impl TablesMut,
) -> Result<Option<VerifiedBlockInformation>, RuntimeError> {
    /* 1. remove block data from tables */

    // Branch on the hard fork version (`major_version`)
    // and add the block to the appropriate table.
    // <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    //
    // FIXME: use `match` with ranges when stable:
    // <https://github.com/rust-lang/rust/issues/37854>
    //
    // TODO: What table to pop from here?
    // Start with v3, if thats empty, try v2, etc?
    if todo!() {
        tables.block_info_v1s_mut().pop_last()?;
    } else if todo!() {
        tables.block_info_v2s_mut().pop_last()?;
    } else {
        tables.block_info_v3s_mut().pop_last()?;
    }

    /* 2. if the caller wants the block info, build it up */
    let option: Option<VerifiedBlockInformation> = if RETURN {
        /* build block */
        let block: VerifiedBlockInformation = todo!();
        Some(block)
    } else {
        None
    };

    Ok(option)
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
