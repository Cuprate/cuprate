//! The blockchain manager interface.
//!
//! This module contains all the functions to mutate the blockchain's state in any way, through the
//! blockchain manager.
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use monero_oxide::{block::Block, transaction::Transaction};
use tokio::sync::{mpsc, oneshot};
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse},
    TxpoolReadHandle,
};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

use crate::{
    blockchain::manager::{BlockchainManagerCommand, IncomingBlockOk},
    constants::PANIC_CRITICAL_SERVICE_ERROR,
};

/// Handle for the blockchain manager.
///
/// Created by [`init_blockchain_manager`](super::manager::init_blockchain_manager).
#[derive(Clone)]
pub struct BlockchainManagerHandle {
    /// The channel used to send [`BlockchainManagerCommand`]s to the blockchain manager.
    command_tx: mpsc::Sender<BlockchainManagerCommand>,
    /// A [`HashSet`] of block hashes that the blockchain manager is currently handling.
    ///
    /// This prevents sending the same block to the blockchain manager from multiple connections
    /// before one of them actually gets added to the chain, allowing peers to do other things.
    ///
    /// This is used over something like a dashmap as we expect a lot of collisions in a short amount of
    /// time for new blocks, so we would lose the benefit of sharded locks. A dashmap is made up of `RwLocks`
    /// which are also more expensive than `Mutex`s.
    blocks_being_handled: Arc<Mutex<HashSet<[u8; 32]>>>,
}

/// An error that can be returned from [`BlockchainManagerHandle::handle_incoming_block`].
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
    /// The blockchain manager command channel is closed.
    #[error("The blockchain manager command channel is closed.")]
    ChannelClosed,
}

impl BlockchainManagerHandle {
    /// Create a new handle and command receiver pair.
    pub fn new() -> (Self, mpsc::Receiver<BlockchainManagerCommand>) {
        let (command_tx, command_rx) = mpsc::channel(3);
        (
            Self {
                command_tx,
                blocks_being_handled: Arc::new(Mutex::new(HashSet::new())),
            },
            command_rx,
        )
    }

    /// Returns `true` if the given block hash is currently being handled.
    pub fn is_block_being_handled(&self, hash: &[u8; 32]) -> bool {
        self.blocks_being_handled.lock().unwrap().contains(hash)
    }

    /// Try to add a new block to the blockchain.
    ///
    /// On success returns `IncomingBlockOk`.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///  - the block was invalid
    ///  - we are missing transactions
    ///  - the block's parent is unknown
    ///  - the blockchain manager command channel is closed
    pub async fn handle_incoming_block(
        &self,
        block: Block,
        mut given_txs: HashMap<[u8; 32], Transaction>,
        blockchain_read_handle: &mut BlockchainReadHandle,
        txpool_read_handle: &mut TxpoolReadHandle,
    ) -> Result<IncomingBlockOk, IncomingBlockError> {
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
                    // We return back the indexes of all txs missing from our pool, not taking into account the txs
                    // that were given with the block, as these txs will be dropped. It is not worth it to try to add
                    // these txs to the pool as this will only happen with a misbehaving peer or if the txpool reaches
                    // the size limit.
                    return Err(IncomingBlockError::UnknownTransactions(block_hash, missing));
                };

                txs.insert(
                    needed_hash,
                    new_tx_verification_data(tx)
                        .map_err(|e| IncomingBlockError::InvalidBlock(e.into()))?,
                );
            }
        }

        // Add the blocks hash to the blocks being handled.
        if !self.blocks_being_handled.lock().unwrap().insert(block_hash) {
            // If another place is already adding this block then we can stop.
            return Ok(IncomingBlockOk::AlreadyHave);
        }

        // We must remove the block hash from `blocks_being_handled`.
        let blocks = Arc::clone(&self.blocks_being_handled);
        let _guard = {
            struct RemoveFromBlocksBeingHandled {
                block_hash: [u8; 32],
                blocks: Arc<Mutex<HashSet<[u8; 32]>>>,
            }
            impl Drop for RemoveFromBlocksBeingHandled {
                fn drop(&mut self) {
                    self.blocks.lock().unwrap().remove(&self.block_hash);
                }
            }
            RemoveFromBlocksBeingHandled { block_hash, blocks }
        };

        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(BlockchainManagerCommand::AddBlock {
                block,
                prepped_txs: txs,
                response_tx,
            })
            .await
            .map_err(|_| IncomingBlockError::ChannelClosed)?;

        response_rx
            .await
            .map_err(|_| IncomingBlockError::ChannelClosed)?
            .map_err(IncomingBlockError::InvalidBlock)
    }

    /// Pop blocks from the top of the blockchain.
    ///
    /// # Errors
    ///
    /// Will error if the blockchain manager channel is closed.
    pub async fn pop_blocks(&self, numb_blocks: usize) -> Result<(), anyhow::Error> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(BlockchainManagerCommand::PopBlocks {
                numb_blocks,
                response_tx,
            })
            .await?;

        Ok(response_rx.await?)
    }
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
