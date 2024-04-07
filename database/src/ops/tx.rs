//! Transactions.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::{Timelock, Transaction};

use cuprate_types::{OutputOnChain, VerifiedBlockInformation};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
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
        Output, PreRctOutputId, RctOutput, TxHash,
    },
};

//---------------------------------------------------------------------------------------------------- Private
/// TODO
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub(super) fn add_tx(
    table_tx_ids: &mut impl DatabaseRw<TxIds>,
    table_heights: &mut impl DatabaseRw<TxHeights>,
    table_unlock_time: &mut impl DatabaseRw<TxUnlockTime>,
) {
    todo!()
}

/// TODO
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub(super) fn remove_tx(
    table_tx_ids: &mut impl DatabaseRw<TxIds>,
    table_heights: &mut impl DatabaseRw<TxHeights>,
    table_unlock_time: &mut impl DatabaseRw<TxUnlockTime>,
) {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// TODO
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[doc = doc_error!()]
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
