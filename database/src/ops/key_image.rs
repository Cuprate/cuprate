//! Spent keys.

//---------------------------------------------------------------------------------------------------- Import
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
        Output, PreRctOutputId, RctOutput, TxHash,
    },
};

//---------------------------------------------------------------------------------------------------- `add_key_image()`
/// TODO
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::key_image::*};
/// // TODO
/// ```
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn add_key_image() -> Result<(), RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `remove_key_image()`
/// TODO
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::key_image::*};
/// // TODO
/// ```
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn remove_key_image() -> Result<(), RuntimeError> {
    todo!()
}

/// TODO
pub fn key_image_exists() {
    todo!()
}
