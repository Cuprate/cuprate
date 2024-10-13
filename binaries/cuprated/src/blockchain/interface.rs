//! The blockchain manager interface.
//!
//! This module contains all the functions to mutate the blockchain's state in any way, through the
//! blockchain manager.
use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex, OnceLock},
};

use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tokio::sync::{mpsc, oneshot};
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_helper::cast::usize_to_u64;
use cuprate_txpool::service::interface::{TxpoolReadRequest, TxpoolReadResponse};
use cuprate_txpool::service::TxpoolReadHandle;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use crate::{
    blockchain::manager::{BlockchainManagerCommand, IncomingBlockOk},
    constants::PANIC_CRITICAL_SERVICE_ERROR,
};

/// The channel used to send [`BlockchainManagerCommand`]s to the blockchain manager.
///
/// This channel is initialized in [`init_blockchain_manager`](super::manager::init_blockchain_manager), the functions
/// in this file document what happens if this is not initialized when they are called.
pub(super) static COMMAND_TX: OnceLock<mpsc::Sender<BlockchainManagerCommand>> = OnceLock::new();

/// An error that can be returned from [`handle_incoming_block`].
#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    /// Some transactions in the block were unknown.
    ///
    /// The inner values are the block hash and the indexes of the missing txs in the block.
    #[error("Unknown transactions in block.")]
    UnknownTransactions([u8; 32], Vec<usize>),
    /// We are missing the block's parent.
    #[error("The block has an unknown parent.")]
    Orphan,
    /// The block was invalid.
    #[error(transparent)]
    InvalidBlock(anyhow::Error),
}

/// Try to add a new block to the blockchain.
///
/// On success returns [`IncomingBlockOk`].
///
/// # Errors
///
/// This function will return an error if:
///  - the block was invalid
///  - we are missing transactions
///  - the block's parent is unknown
pub async fn handle_incoming_block(
    block: Block,
    mut given_txs: HashMap<[u8; 32], Transaction>,
    blockchain_read_handle: &mut BlockchainReadHandle,
    txpool_read_handle: &mut TxpoolReadHandle,
) -> Result<IncomingBlockOk, IncomingBlockError> {
    /// A [`HashSet`] of block hashes that the blockchain manager is currently handling.
    ///
    /// This lock prevents sending the same block to the blockchain manager from multiple connections
    /// before one of them actually gets added to the chain, allowing peers to do other things.
    ///
    /// This is used over something like a dashmap as we expect a lot of collisions in a short amount of
    /// time for new blocks, so we would lose the benefit of sharded locks. A dashmap is made up of `RwLocks`
    /// which are also more expensive than `Mutex`s.
    static BLOCKS_BEING_HANDLED: LazyLock<Mutex<HashSet<[u8; 32]>>> =
        LazyLock::new(|| Mutex::new(HashSet::new()));

    if given_txs.len() > block.transactions.len() {
        return Err(IncomingBlockError::InvalidBlock(anyhow::anyhow!(
            "Too many transactions given for block"
        )));
    }

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
        return Ok(IncomingBlockOk::AlreadyHave);
    }

    let TxpoolReadResponse::TxsForBlock { mut txs, missing } = txpool_read_handle
        .ready()
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
        .call(TxpoolReadRequest::TxsForBlock(block.transactions.clone()))
        .await
        .expect(PANIC_CRITICAL_SERVICE_ERROR)
    else {
        unreachable!()
    };

    if !missing.is_empty() {
        let needed_hashes = missing.iter().map(|index| block.transactions[*index]);

        for needed_hash in needed_hashes {
            let Some(tx) = given_txs.remove(&needed_hash) else {
                return Err(IncomingBlockError::UnknownTransactions(block_hash, missing));
            };

            txs.insert(
                needed_hash,
                new_tx_verification_data(tx)
                    .map_err(|e| IncomingBlockError::InvalidBlock(e.into()))?,
            );
        }
    }

    let Some(incoming_block_tx) = COMMAND_TX.get() else {
        // We could still be starting up the blockchain manager.
        return Ok(IncomingBlockOk::NotReady);
    };

    // Add the blocks hash to the blocks being handled.
    if !BLOCKS_BEING_HANDLED.lock().unwrap().insert(block_hash) {
        // If another place is already adding this block then we can stop.
        return Ok(IncomingBlockOk::AlreadyHave);
    }

    // From this point on we MUST not early return without removing the block hash from `BLOCKS_BEING_HANDLED`.

    let (response_tx, response_rx) = oneshot::channel();

    incoming_block_tx
        .send(BlockchainManagerCommand::AddBlock {
            block,
            prepped_txs: txs,
            response_tx,
        })
        .await
        .expect("TODO: don't actually panic here, an err means we are shutting down");

    let res = response_rx
        .await
        .expect("The blockchain manager will always respond")
        .map_err(IncomingBlockError::InvalidBlock);

    // Remove the block hash from the blocks being handled.
    BLOCKS_BEING_HANDLED.lock().unwrap().remove(&block_hash);

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
        unreachable!();
    };

    Ok(chain.is_some())
}
