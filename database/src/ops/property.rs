//! Properties.

//---------------------------------------------------------------------------------------------------- Import
use monero_pruning::PruningSeed;
use monero_serai::transaction::{Timelock, Transaction};

use cuprate_types::{OutputOnChain, VerifiedBlockInformation};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::macros::{doc_add_block_inner_invariant, doc_error},
    tables::{
        BlockBlobs, BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s, KeyImages, NumOutputs,
        Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut,
        TxHeights, TxIds, TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        BlockHash, BlockHeight, BlockInfoLatest, BlockInfoV1, BlockInfoV2, BlockInfoV3, KeyImage,
        Output, PreRctOutputId, RctOutput, TxHash, TxId,
    },
};
//---------------------------------------------------------------------------------------------------- Free Functions
/// TODO
///
/// TODO: document this add to the latest block height.
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[inline]
pub const fn get_blockchain_pruning_seed() -> Result<PruningSeed, RuntimeError> {
    // TODO: impl pruning.
    // We need a DB properties table.
    Ok(PruningSeed::NotPruned)
}
