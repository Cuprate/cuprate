//! Blockchain.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::Timelock;

use cuprate_types::VerifiedBlockInformation;

use crate::{
    database::{DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::macros::doc_error,
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
/// Retrieve the height of the chain.
///
/// This returns the chain-tip, not the height of top block.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn chain_height(
    table_block_heights: &impl DatabaseRo<BlockHeights>,
) -> Result<BlockHeight, RuntimeError> {
    table_block_heights.len()
}
