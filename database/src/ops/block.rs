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
        Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        BlockHash, BlockHeight, BlockInfoLatest, BlockInfoV1, BlockInfoV2, BlockInfoV3, KeyImage,
        Output, PreRctOutputId, RctOutput,
    },
};

//---------------------------------------------------------------------------------------------------- Free functions
/// TODO
///
/// # Errors
/// TODO
#[inline]
#[allow(clippy::cast_possible_truncation)] // TODO: remove me
pub fn add_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    block: VerifiedBlockInformation,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    // Branch on the hard fork version (`major_version`)
    // and add the block to the appropriate table.
    // <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    //
    // FIXME: use `match` with ranges when stable:
    // <https://github.com/rust-lang/rust/issues/37854>
    if block.block.header.major_version < 4 {
        env.open_db_rw::<BlockInfoV1s>(tx_rw)?.put(
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
        env.open_db_rw::<BlockInfoV2s>(tx_rw)?.put(
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
        env.open_db_rw::<BlockInfoV3s>(tx_rw)?.put(
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
    env.open_db_rw::<BlockBlobs>(tx_rw)?
        .put(&block.height, todo!())?;

    // BlockHeights: BlockHash => BlockHeight
    env.open_db_rw::<BlockHeights>(tx_rw)?
        .put(&block.block_hash, &block.height)?;

    // KeyImages: KeyImage > ()
    {
        let mut table = env.open_db_rw::<KeyImages>(tx_rw)?;
        let key_images: std::slice::Iter<'_, KeyImage> = todo!();
        for key_image in key_images {
            table.put(key_image, &())?;
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
        // FIXME: These will have to be re-opened write
        // table since `open_db_rw` takes `&mut TxRw`.
        let mut table_num_outputs = env.open_db_rw::<NumOutputs>(tx_rw)?;
        let mut table_pruned_tx_blobs = env.open_db_rw::<PrunedTxBlobs>(tx_rw)?;
        let mut table_prunable_hashes = env.open_db_rw::<PrunableHashes>(tx_rw)?;
        let mut table_outputs = env.open_db_rw::<Outputs>(tx_rw)?;
        let mut table_prunable_tx_blobs = env.open_db_rw::<PrunableTxBlobs>(tx_rw)?;
        let mut table_rct_outputs = env.open_db_rw::<RctOutputs>(tx_rw)?;
        let mut table_tx_ids = env.open_db_rw::<TxIds>(tx_rw)?;
        let mut table_tx_heights = env.open_db_rw::<TxHeights>(tx_rw)?;
        let mut table_tx_unlock_time = env.open_db_rw::<TxUnlockTime>(tx_rw)?;

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

            table_pruned_tx_blobs.put(&tx_id, prunable_blob)?;
            table_prunable_hashes.put(&tx_id, prunable_hash)?;
            table_rct_outputs.put(&tx_id, &rct_output)?;

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

                table_num_outputs.put(&amount, &amount_index)?;
                table_outputs.put(&pre_rct_output_id, &output)?;
                table_tx_ids.put(&tx.tx_hash, &tx_id)?;
                table_tx_heights.put(&tx_id, &block.height)?;

                let unlock_time = match tx.tx.prefix.timelock {
                    Timelock::None => todo!(),
                    Timelock::Block(height) => todo!(), // Calculate from height?
                    Timelock::Time(time) => time,
                };
                table_tx_unlock_time.put(&tx_id, &unlock_time)?;
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
pub fn add_blocks<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height_and_blocks: &[(BlockHeight, BlockInfoLatest)],
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    todo!()
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn pop_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
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
    tx_rw: &mut Rw,
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
