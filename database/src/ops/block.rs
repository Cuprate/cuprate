//! Blocks.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    database::{DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    tables::{BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s},
    transaction::{TxRo, TxRw},
    types::{BlockHash, BlockHeight, BlockInfoLatest, BlockInfoV1, BlockInfoV2, BlockInfoV3},
};

//---------------------------------------------------------------------------------------------------- Free functions
/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_block<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height: BlockHeight,
    block: &BlockInfoLatest,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    add_block_v3(env, tx_rw, height, block)
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
    add_blocks_v3(env, tx_rw, height_and_blocks)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_block_v1<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height: BlockHeight,
    block: &BlockInfoV1,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_rw::<BlockInfoV1s>(tx_rw)?.put(&height, block)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_blocks_v1<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height_and_blocks: &[(BlockHeight, BlockInfoV1)],
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let mut table = env.open_db_rw::<BlockInfoV1s>(tx_rw)?;
    for (height, block) in height_and_blocks {
        table.put(height, block)?;
    }
    Ok(())
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_block_v2<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height: BlockHeight,
    block: &BlockInfoV2,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_rw::<BlockInfoV2s>(tx_rw)?.put(&height, block)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_blocks_v2<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height_and_blocks: &[(BlockHeight, BlockInfoV2)],
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let mut table = env.open_db_rw::<BlockInfoV2s>(tx_rw)?;
    for (height, block) in height_and_blocks {
        table.put(height, block)?;
    }
    Ok(())
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_block_v3<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height: BlockHeight,
    block: &BlockInfoV3,
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    env.open_db_rw::<BlockInfoV3s>(tx_rw)?.put(&height, block)
}

/// TODO
///
/// # Errors
/// TODO
#[inline]
pub fn add_blocks_v3<'env, Ro, Rw, Env>(
    env: &Env,
    tx_rw: &mut Rw,
    height_and_blocks: &[(BlockHeight, BlockInfoV3)],
) -> Result<(), RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let mut table = env.open_db_rw::<BlockInfoV3s>(tx_rw)?;
    for (height, block) in height_and_blocks {
        table.put(height, block)?;
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
