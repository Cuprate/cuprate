//! The blockchain manger interface.
//!
//! This module contains all the functions to mutate the blockchains state in any way, through the
//! blockchain manger.
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tokio::sync::{mpsc, oneshot};
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_helper::cast::usize_to_u64;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use crate::{
    blockchain::manager::BlockchainManagerCommand, constants::PANIC_CRITICAL_SERVICE_ERROR,
};

/// The channel used to send [`BlockchainManagerCommand`]s to the blockchain manger.
pub static COMMAND_TX: OnceLock<mpsc::Sender<BlockchainManagerCommand>> = OnceLock::new();

/// A [`HashSet`] of block hashes that the blockchain manager is currently handling.
pub static BLOCKS_BEING_HANDLED: OnceLock<Mutex<HashSet<[u8; 32]>>> = OnceLock::new();

/// An error that can be returned from [`handle_incoming_block`].
#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    /// Some transactions in the block were unknown.
    ///
    /// The inner values are the block hash and the indexes of the missing txs in the block.
    #[error("Unknown transactions in block.")]
    UnknownTransactions([u8; 32], Vec<u64>),
    /// We are missing the block's parent.
    #[error("The block has an unknown parent.")]
    Orphan,
    /// The block was invalid.
    #[error(transparent)]
    InvalidBlock(anyhow::Error),
}

/// Try to add a new block to the blockchain.
///
/// This returns a [`bool`] indicating if the block was added to the main-chain ([`true`]) or an alt-chain
/// ([`false`]).
///
/// If we already knew about this block or the blockchain manger is not setup yet `Ok(false)` is returned.
///
/// # Errors
///
/// This function will return an error if:
///  - the block was invalid
///  - we are missing transactions
///  - the block's parent is unknown
pub async fn handle_incoming_block(
    block: Block,
    given_txs: Vec<Transaction>,
    blockchain_read_handle: &mut BlockchainReadHandle,
) -> Result<bool, IncomingBlockError> {
    // FIXME: we should look in the tx-pool for txs when that is ready.

    if !block_exists(block.header.previous, blockchain_read_handle)
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
    {
        return Err(IncomingBlockError::Orphan);
    }

    let block_hash = block.hash();

    if block_exists(block_hash, blockchain_read_handle)
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
    {
        return Ok(false);
    }

    // TODO: remove this when we have a working tx-pool.
    if given_txs.len() != block.transactions.len() {
        return Err(IncomingBlockError::UnknownTransactions(
            block_hash,
            (0..usize_to_u64(block.transactions.len())).collect(),
        ));
    }

    // TODO: check we actually go given the right txs.
    let prepped_txs = given_txs
        .into_par_iter()
        .map(|tx| {
            let tx = new_tx_verification_data(tx)?;
            Ok((tx.tx_hash, tx))
        })
        .collect::<Result<_, anyhow::Error>>()
        .map_err(IncomingBlockError::InvalidBlock)?;

    let Some(incoming_block_tx) = COMMAND_TX.get() else {
        // We could still be starting up the blockchain manger, so just return this as there is nothing
        // else we can do.
        return Ok(false);
    };

    // Add the blocks hash to the blocks being handled.
    if !BLOCKS_BEING_HANDLED
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap()
        .insert(block_hash)
    {
        // If another place is already adding this block then we can stop.
        return Ok(false);
    }

    // From this point on we MUST not early return without removing the block hash from `BLOCKS_BEING_HANDLED`.

    let (response_tx, response_rx) = oneshot::channel();

    incoming_block_tx
        .send(BlockchainManagerCommand::AddBlock {
            block,
            prepped_txs,
            response_tx,
        })
        .await
        .expect("TODO: don't actually panic here, an err means we are shutting down");

    let res = response_rx
        .await
        .expect("The blockchain manager will always respond")
        .map_err(IncomingBlockError::InvalidBlock);

    // Remove the block hash from the blocks being handled.
    BLOCKS_BEING_HANDLED
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .remove(&block_hash);

    res
}

/// Check if we have a block with the given hash.
async fn block_exists(
    block_hash: [u8; 32],
    blockchain_read_handle: &mut BlockchainReadHandle,
) -> Result<bool, anyhow::Error> {
    let BlockchainResponse::FindBlock(chain) = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::FindBlock(block_hash))
        .await?
    else {
        panic!("Invalid blockchain response!");
    };

    Ok(chain.is_some())
}
