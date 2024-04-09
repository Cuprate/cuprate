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
pub fn add_tx(tx: &Transaction, tables: &mut impl TablesMut) -> Result<TxId, RuntimeError> {
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

    // TODO: implement pruning after `monero-serai` does.
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // TODO: what to store here? which table?
    // }

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
pub fn remove_tx(tx_hash: &TxHash, tables: &mut impl TablesMut) -> Result<TxId, RuntimeError> {
    let tx_id = tables.tx_ids_mut().take(tx_hash)?;
    tables.tx_heights_mut().delete(&tx_id)?;

    // TODO: implement pruning after `monero-serai` does.
    // table_prunable_hashes.delete(&tx_id)?;
    // table_prunable_tx_blobs.delete(&tx_id)?;
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // TODO: what to remove here? which table?
    // }

    match tables.tx_unlock_time_mut().delete(&tx_id) {
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
