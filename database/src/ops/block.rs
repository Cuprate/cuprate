//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::Timelock;

use cuprate_types::VerifiedBlockInformation;

use crate::{
    database::{DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
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
/// Consider using [`add_block_bulk()`] for multiple blocks.
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
#[allow(clippy::too_many_lines)]
pub fn add_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
    block: &VerifiedBlockInformation,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    add_block_inner(&mut env.open_tables_mut(tx_rw)?, block)
}

/// Bulk version of [`add_block()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn add_block_bulk<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
    blocks: impl Iterator<Item = &'env VerifiedBlockInformation>,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let tables = &mut env.open_tables_mut(tx_rw)?;
    for block in blocks {
        add_block_inner(tables, block)?;
    }
    Ok(())
}

/// Internal function used by:
/// - [`add_block()`]
/// - [`add_blocks()`]
#[allow(clippy::cast_possible_truncation)] // TODO: remove me
fn add_block_inner(
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
                cumulative_difficulty: block.cumulative_difficulty as u64, // TODO
                block_hash: block.block_hash,
            },
        )
    } else if block.block.header.major_version < 10 {
        tables.block_info_v2s_mut().put(
            &block.height,
            &BlockInfoV2 {
                timestamp: block.block.header.timestamp,
                total_generated_coins: block.generated_coins,
                weight: block.weight as u64, // TODO
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

    // KeyImages: KeyImage > ()
    {
        let key_images: std::slice::Iter<'_, KeyImage> = todo!();
        for key_image in key_images {
            tables.key_images_mut().put(key_image, &())?;
        }
    }

    // Transaction data.
    //
    // - NumOutputs:      Amount         => AmountIndex
    // - PrunedTxBlobs:   TxId           => PrunableBlob
    // - PrunableHashes:  TxId           => PrunableHash
    // - Outputs:         PreRctOutputId => Output
    // - PrunableTxBlobs: TxId           => PrunableBlob
    // - RctOutputs:      AmountIndex    => RctOutput
    // - TxIds:           TxHash         => TxId
    {
        for tx in block.txs {
            let tx_id = todo!();
            let prunable_blob = todo!();
            let prunable_hash = todo!();
            let rct_output = RctOutput {
                key: todo!(),
                height: todo!(),
                output_flags: todo!(),
                tx_idx: todo!(),
                commitment: todo!(),
            };

            tables.pruned_tx_blobs_mut().put(&tx_id, prunable_blob)?;
            tables.prunable_hashes_mut().put(&tx_id, prunable_hash)?;
            tables.rct_outputs_mut().put(&tx_id, &rct_output)?;

            for output in tx.tx.prefix.outputs {
                let amount = todo!();
                let amount_index = todo!();

                let pre_rct_output_id = PreRctOutputId {
                    amount,
                    amount_index,
                };

                let output = Output {
                    key: *output.key.as_bytes(),
                    height: todo!(),
                    output_flags: todo!(),
                    tx_idx: todo!(),
                };

                tables.num_outputs_mut().put(&amount, &amount_index)?;
                tables.outputs_mut().put(&pre_rct_output_id, &output)?;
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
/// - Consider using [`pop_block_bulk()`] for multiple blocks
/// - Consider using [`pop_block_cheap()`] if the returned block is unneeded
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
#[allow(clippy::missing_panics_doc)] // this should not panic
pub fn pop_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
) -> Result<VerifiedBlockInformation, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    Ok(pop_block_inner::<true>(&mut env.open_tables_mut(tx_rw)?)?
        .expect("this will always return `Some`"))
}

/// Bulk version of [`pop_block()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
#[allow(clippy::missing_panics_doc)] // this should not panic
pub fn pop_block_bulk<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
    count: usize,
) -> Result<Vec<VerifiedBlockInformation>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let tables = &mut env.open_tables_mut(tx_rw)?;
    let mut blocks = Vec::with_capacity(count);

    for i in 0..count {
        let block = pop_block_inner::<true>(tables)?.expect("this will always return `Some`");
        blocks.push(block);
    }

    Ok(blocks)
}

/// A cheaper to call [`pop_block()`].
///
/// This is the same as `pop_block()` however it will
/// not construct and return the block removed, thus,
/// it should be faster to call in situations where the
/// returned block would not be used anyway.
///
/// Consider using [`pop_block_cheap_bulk()`] for multiple blocks.
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn pop_block_cheap<'env, Ro, Rw, Env>(env: &Env, tx_rw: &Rw) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let option = pop_block_inner::<false>(&mut env.open_tables_mut(tx_rw)?)?;
    debug_assert!(option.is_none());
    Ok(())
}

/// Bulk version of [`pop_block_cheap()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn pop_block_cheap_bulk<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
    count: usize,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let tables = &mut env.open_tables_mut(tx_rw)?;

    for i in 0..count {
        let option = pop_block_inner::<false>(tables)?;
        debug_assert!(option.is_none());
    }

    Ok(())
}

/// Internal function that is used by:
/// - [`pop_block()`]
/// - [`pop_block_bulk()`]
/// - [`pop_block_cheap()`]
/// - [`pop_block_cheap_bulk()`]
///
/// The `const RETURN: bool` will dictate if this function
/// returns the block wrapped in `Some` or `None`.
///
/// # Invariant
/// - `RETURN == true` -> This must return `Some`
/// - `RETURN == false` -> This must return `None`
///
/// # Errors
/// TODO
#[inline]
pub fn pop_block_inner<const RETURN: bool>(
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
/// Retrieve a [`VerifiedBlockInformation`] to the database.
///
/// This extracts all the data from the database needed
/// to create a full `VerifiedBlockInformation`.
///
/// Consider using [`get_block_bulk()`] for multiple blocks.
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn get_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    height: BlockHeight,
) -> Result<VerifiedBlockInformation, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    get_block_inner(&env.open_tables(tx_ro)?, height)
}

/// Bulk version of [`get_block()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_bulk<'env, Ro, Rw, Env, Heights>(
    env: &Env,
    tx_ro: &Ro,
    heights: Heights,
) -> Result<Vec<VerifiedBlockInformation>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
    Heights: Iterator<Item = &'env BlockHeight> + ExactSizeIterator,
{
    let tables = &env.open_tables(tx_ro)?;
    let mut blocks = Vec::with_capacity(heights.len());

    for height in heights {
        let block = get_block_inner(tables, *height)?;
        blocks.push(block);
    }

    Ok(blocks)
}

/// Internal function that is used by:
/// - [`get_block()`]
/// - [`get_block_bulk()`]
///
/// # Errors
/// TODO
#[inline]
fn get_block_inner(
    tables: &impl Tables,
    height: BlockHeight,
) -> Result<VerifiedBlockInformation, RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_block_top_*`
/// Return the top block from the database.
///
/// This is the same as [`pop_block()`], but it does
/// not remove the block, it only retrieves it.
///
/// - Consider using [`get_block_top_bulk()`] for multiple blocks
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_top<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
) -> Result<VerifiedBlockInformation, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let tables = &env.open_tables(tx_ro)?;
    get_block_inner(tables, top_block_height(tables)?)
}

/// Bulk version of [`get_block_top()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_top_bulk<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    count: u64,
) -> Result<Vec<VerifiedBlockInformation>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let tables = &env.open_tables(tx_ro)?;

    let top_block_height = top_block_height(tables)?;
    if count > top_block_height {
        // Caller asked for more blocks than we have.
        return todo!();
    }

    #[allow(clippy::cast_possible_truncation)] // TODO
    let mut vec = Vec::with_capacity(count as usize);

    for height in count..=top_block_height {
        let block = get_block_inner(tables, height)?;
        vec.push(block);
    }

    Ok(vec)
}

//---------------------------------------------------------------------------------------------------- `get_block_height_*`
/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_height<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    block_hash: &BlockHash,
) -> Result<BlockHeight, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    // No need to open all tables, just 1.
    env.open_db_ro::<BlockHeights>(tx_ro)?.get(block_hash)
}

/// Bulk version of [`get_block_height()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_height_bulk<'env, Ro, Rw, Env, Hashes>(
    env: &Env,
    tx_ro: &Ro,
    block_hashes: Hashes,
) -> Result<Vec<BlockHeight>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
    Hashes: Iterator<Item = &'env BlockHash> + ExactSizeIterator,
{
    // FIXME: can we return an `impl Iterator` here instead of `Vec`?

    let table_block_heights = env.open_db_ro::<BlockHeights>(tx_ro)?;
    let mut heights = Vec::with_capacity(block_hashes.len());

    for block_hash in block_hashes {
        let height = table_block_heights.get(block_hash)?;
        heights.push(height);
    }

    Ok(heights)
}

//---------------------------------------------------------------------------------------------------- Misc
/// TODO
///
/// # Errors
/// TODO
#[inline]
fn top_block_height(tables: &impl Tables) -> Result<BlockHeight, RuntimeError> {
    // TODO: is this correct?
    tables.block_heights().len()
}

/// Check if a block does _NOT_ exist in the database.
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn block_missing<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    block_hash: &BlockHash,
) -> Result<bool, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_ro::<BlockHeights>(tx_ro)?.contains(block_hash)
}

/// Bulk version of [`block_missing()`].
///
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn block_missing_bulk<'env, Ro, Rw, Env, Hashes>(
    env: &Env,
    tx_ro: &Ro,
    block_hashes: Hashes,
) -> Result<Vec<&'env BlockHash>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
    Hashes: Iterator<Item = &'env BlockHash> + ExactSizeIterator,
{
    // FIXME: can we return an `impl Iterator` here instead of `Vec`?

    let table_block_heights = env.open_db_ro::<BlockHeights>(tx_ro)?;
    let mut blocks = Vec::with_capacity(block_hashes.len());

    for block_hash in block_hashes {
        if !table_block_heights.contains(block_hash)? {
            blocks.push(block_hash);
        }
    }

    Ok(blocks)
}
