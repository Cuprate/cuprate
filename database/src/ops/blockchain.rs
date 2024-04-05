//! Blockchain.

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

//---------------------------------------------------------------------------------------------------- Free Functions
/// Retrieve the block height of the latest/top block in the database.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
///
/// # Errors
/// TODO
#[inline]
pub fn height<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    count: u64,
) -> Result<BlockHeight, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    let table_block_heights = env.open_db_ro::<BlockHeights>(tx_ro)?;
    height_internal(&table_block_heights)
}

/// Internal function for [`height()`].
///
/// - Re-used elsewhere
/// - More efficient as it takes the single table it needs directly
#[inline]
pub(super) fn height_internal(
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<BlockHeight, RuntimeError> {
    // TODO: is this correct?
    // TODO: is there a faster way to do this? `.len()` is already quite cheap.
    table_block_heights.len()
}
