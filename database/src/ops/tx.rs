//! Transactions.

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

//---------------------------------------------------------------------------------------------------- Private
/// TODO
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn add_tx(
    table_tx_ids: &mut impl DatabaseRw<TxIds>,
    table_heights: &mut impl DatabaseRw<TxHeights>,
    table_unlock_time: &mut impl DatabaseRw<TxUnlockTime>,
) -> Result<(), RuntimeError> {
    todo!()
}

/// TODO
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn remove_tx(
    table_tx_ids: &mut impl DatabaseRw<TxIds>,
    table_heights: &mut impl DatabaseRw<TxHeights>,
    table_unlock_time: &mut impl DatabaseRw<TxUnlockTime>,
) -> Result<(), RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// TODO
///
#[doc = doc_error!()]
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[inline]
pub fn get_tx(
    table_tx_ids: &(impl DatabaseRo<TxIds> + DatabaseIter<TxIds>),
    table_heights: &(impl DatabaseRo<TxHeights> + DatabaseIter<TxHeights>),
    table_unlock_time: &(impl DatabaseRo<TxUnlockTime> + DatabaseIter<TxUnlockTime>),
    tx_hash: TxHash,
) -> Result<Transaction, RuntimeError> {
    todo!()
}

//----------------------------------------------------------------------------------------------------
/// TODO
pub fn get_num_tx() {
    todo!()
}

/// TODO
pub fn tx_exists() {
    todo!()
}

/// TODO
pub fn get_tx_unlock_time() {
    todo!()
}

/// TODO
pub fn get_tx_list() {
    todo!()
}

/// TODO
pub fn get_pruned_tx() {
    todo!()
}

/// TODO
pub fn get_tx_block_height() {
    todo!()
}
