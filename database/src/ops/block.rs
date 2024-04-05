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

//---------------------------------------------------------------------------------------------------- Private free functions
// Some utility functions/types used internally in the public functions.
//---------------------------------------------------------------------------------------------------- Free functions
/// TODO
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

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_blocks<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
    blocks: &[VerifiedBlockInformation],
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let mut tables = env.open_tables_mut(tx_rw)?;
    for block in blocks {
        add_block_inner(&mut tables, block)?;
    }
    Ok(())
}

/// Internal function that [`add_block()`] and [`add_blocks()`] uses.
///
/// It takes the already opened tables.
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

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn pop_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
) -> Result<(BlockHeight, BlockInfoLatest), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    todo!(); // remove related info?
    env.open_db_rw::<BlockInfoV3s>(tx_rw)?.pop_last()
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn pop_blocks<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &Rw,
    count: usize,
) -> Result<Vec<(BlockHeight, BlockInfoLatest)>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    todo!(); // remove related info?

    let mut table = env.open_db_rw::<BlockInfoV3s>(tx_rw)?;

    let mut vec = Vec::with_capacity(count);

    for _ in 0..count {
        let (height, block) = match table.pop_last() {
            Ok(tuple) => tuple,
            Err(RuntimeError::KeyNotFound) => return Ok(vec),
            Err(error) => return Err(error),
        };
        vec.push((height, block));
    }

    Ok(vec)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn get_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    height: BlockHeight,
) -> Result<BlockInfoLatest, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    get_block_v3(env, tx_ro, height)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_v1<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    height: BlockHeight,
) -> Result<BlockInfoV1, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_ro::<BlockInfoV1s>(tx_ro)?.get(&height)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_v2<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    height: BlockHeight,
) -> Result<BlockInfoV2, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_ro::<BlockInfoV2s>(tx_ro)?.get(&height)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn get_block_v3<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    height: BlockHeight,
) -> Result<BlockInfoV3, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_ro::<BlockInfoV3s>(tx_ro)?.get(&height)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn get_top_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
) -> Result<(BlockHeight, BlockInfoLatest), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_ro::<BlockInfoV3s>(tx_ro)?.last()
}

/// TODO
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
    env.open_db_ro::<BlockHeights>(tx_ro)?.get(block_hash)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn block_exists<'env, Ro, Rw, Env>(
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

/// TODO
pub fn get_block_header() {
    todo!()
}

/// TODO
pub fn get_block_header_from_height() {
    todo!()
}

/// TODO
pub fn get_top_block_hash() {
    todo!()
}

/// TODO
pub fn get_block_hash() {
    todo!()
}

/// TODO
pub fn get_block_weight() {
    todo!()
}

/// TODO
pub fn get_block_already_generated_coins() {
    todo!()
}

/// TODO
pub fn get_block_long_term_weight() {
    todo!()
}

/// TODO
pub fn get_block_timestamp() {
    todo!()
}

/// TODO
pub fn get_block_cumulative_rct_outputs() {
    todo!()
}

/// TODO
pub fn get_block_from_height() {
    todo!()
}
