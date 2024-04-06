//! Transactions.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::{Timelock, Transaction};

use cuprate_types::{OutputOnChain, VerifiedBlockInformation};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::{
        blockchain::height_internal,
        macros::{doc_error, doc_fn},
    },
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
#[doc = doc_fn!(get_tx_bulk)]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_tx<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    tx_hash: TxHash,
) -> Result<Transaction, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    get_tx_internal(
        tx_hash,
        &env.open_db_ro::<TxIds>(tx_ro)?,
        &env.open_db_ro::<TxHeights>(tx_ro)?,
        &env.open_db_ro::<TxUnlockTime>(tx_ro)?,
    )
}

/// TODO
///
#[doc = doc_fn!(get_tx, bulk)]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[doc = doc_error!(bulk)]
#[inline]
pub fn get_tx_bulk<'env, Ro, Rw, Env, Iter>(
    env: &Env,
    tx_ro: &Ro,
    tx_hashes: Iter,
) -> Result<Vec<Transaction>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
    Iter: Iterator<Item = TxHash> + ExactSizeIterator,
{
    let (table_tx_ids, table_heights, table_unlock_time) = (
        &env.open_db_ro::<TxIds>(tx_ro)?,
        &env.open_db_ro::<TxHeights>(tx_ro)?,
        &env.open_db_ro::<TxUnlockTime>(tx_ro)?,
    );

    let mut vec = Vec::with_capacity(tx_hashes.len());

    for tx_hash in tx_hashes {
        let tx = get_tx_internal(tx_hash, table_tx_ids, table_heights, table_unlock_time)?;
        vec.push(tx);
    }

    Ok(vec)
}

/// TODO
#[inline]
pub(super) fn get_tx_internal(
    tx_hash: TxHash,
    table_tx_ids: &(impl DatabaseRo<TxIds> + DatabaseIter<TxIds>),
    table_heights: &(impl DatabaseRo<TxHeights> + DatabaseIter<TxHeights>),
    table_unlock_time: &(impl DatabaseRo<TxUnlockTime> + DatabaseIter<TxUnlockTime>),
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
