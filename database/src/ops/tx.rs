//! Transactions.

//---------------------------------------------------------------------------------------------------- Import
use cuprate_types::{OutputOnChain, TransactionVerificationData, VerifiedBlockInformation};
use monero_pruning::PruningSeed;
use monero_serai::transaction::{Timelock, Transaction};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::macros::{doc_add_block_inner_invariant, doc_error},
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        BlockHash, BlockHeight, BlockInfo, KeyImage, Output, PreRctOutputId, RctOutput, TxHash,
        TxId,
    },
};

use super::property::get_blockchain_pruning_seed;

//---------------------------------------------------------------------------------------------------- Private
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn add_tx(
    tx: &Transaction,
    tables: &mut impl TablesMut,
    // table_block_heights: &impl DatabaseRo<BlockHeights>,
    // table_tx_ids: &mut impl DatabaseRw<TxIds>,
    // table_tx_heights: &mut impl DatabaseRw<TxHeights>,
    // table_tx_unlock_time: &mut impl DatabaseRw<TxUnlockTime>,
    // table_prunable_hashes: &mut impl DatabaseRw<PrunableHashes>,
    // table_prunable_tx_blobs: &mut impl DatabaseRw<PrunableTxBlobs>,
) -> Result<TxId, RuntimeError> {
    let tx_id = get_num_tx(tables.tx_ids_mut())?;
    let height = crate::ops::blockchain::chain_height(tables.block_heights_mut())?;

    tables.tx_ids_mut().put(&tx.hash(), &tx_id)?;
    tables.tx_heights_mut().put(&tx_id, &height)?;

    // TODO: What exactly is a `UnlockTime (u64)` in Cuprate's case?
    // What should we be storing? How?
    match tx.prefix.timelock {
        Timelock::None => (),
        Timelock::Block(height) => tables.tx_unlock_time_mut().put(&tx_id, &(height as u64))?,
        Timelock::Time(time) => tables.tx_unlock_time_mut().put(&tx_id, &time)?,
    }

    // TODO: is this the correct field? how should it be hashed?
    let prunable_hash = /* hash_fn(tx_rct_signatures.prunable) */ todo!();
    tables.prunable_hashes_mut().put(&tx_id, &prunable_hash)?;
    // TODO: what part of `tx` is prunable?
    //
    // `tx.prefix: TransactionPrefix` + `tx.rct_signatures.prunable: RctPrunable`
    // combined as a `StorableVec`?
    //
    // Is it `tx.blob: Vec<u8>`?
    tables.prunable_tx_blobs_mut().put(&tx_id, todo!())?;

    // TODO: impl pruning
    if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
        // TODO: what to store here? which table?
    }

    Ok(tx_id)
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
    tx_hash: &TxHash,
    table_tx_ids: &mut impl DatabaseRw<TxIds>,
    table_heights: &mut impl DatabaseRw<TxHeights>,
    table_unlock_time: &mut impl DatabaseRw<TxUnlockTime>,
    table_prunable_hashes: &mut impl DatabaseRw<PrunableHashes>,
    table_prunable_tx_blobs: &mut impl DatabaseRw<PrunableTxBlobs>,
) -> Result<TxId, RuntimeError> {
    let tx_id = table_tx_ids.get(tx_hash)?;
    table_tx_ids.delete(tx_hash)?;
    table_heights.delete(&tx_id)?;
    table_prunable_hashes.delete(&tx_id)?;
    table_prunable_tx_blobs.delete(&tx_id)?;

    // TODO: impl pruning
    if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
        // TODO: what to remove here? which table?
    }

    match table_unlock_time.delete(&tx_id) {
        Err(RuntimeError::KeyNotFound) | Ok(()) => (),
        // An actual error occurred, return.
        Err(e) => return Err(e),
    };

    Ok(tx_id)
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
    tx_hash: &TxHash,
    table_tx_ids: &(impl DatabaseRo<TxIds> + DatabaseIter<TxIds>),
    table_heights: &(impl DatabaseRo<TxHeights> + DatabaseIter<TxHeights>),
    table_unlock_time: &(impl DatabaseRo<TxUnlockTime> + DatabaseIter<TxUnlockTime>),
) -> Result<Transaction, RuntimeError> {
    todo!()
}

//----------------------------------------------------------------------------------------------------
/// TODO
///
#[doc = doc_error!()]
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[inline]
pub fn get_num_tx(table_tx_ids: &impl DatabaseRo<TxIds>) -> Result<u64, RuntimeError> {
    table_tx_ids.len()
}

//----------------------------------------------------------------------------------------------------
/// TODO
///
#[doc = doc_error!()]
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[inline]
pub fn tx_exists(
    tx_hash: &TxHash,
    table_tx_ids: &impl DatabaseRo<TxIds>,
) -> Result<bool, RuntimeError> {
    table_tx_ids.contains(tx_hash)
}

/// TODO
pub fn get_pruned_tx() {
    todo!()
}
