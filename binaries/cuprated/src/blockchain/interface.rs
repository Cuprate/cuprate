//! The blockchain manager interface.
//!
//! This module contains all the functions to mutate the blockchain's state in any way, through the
//! blockchain manager.
use std::collections::HashMap;

use monero_oxide::{block::Block, transaction::Transaction};
use tokio::sync::{mpsc, oneshot};
use tower::{Service, ServiceExt};

use cuprate_blockchain::{service::BlockchainReadHandle, BlockchainError};
use cuprate_consensus::{block::BlockVerificationError, transactions::new_tx_verification_data};
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse},
    TxpoolReadHandle,
};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

use crate::blockchain::{
    known_blocks::KnownBlocks,
    manager::{BlockchainManagerCommand, IncomingBlockOk},
    IncomingBlockError,
};

/// Handle for the blockchain manager.
///
/// Created by `init_blockchain_manager`.
#[derive(Clone)]
pub struct BlockchainManagerHandle {
    /// The channel used to send [`BlockchainManagerCommand`]s to the blockchain manager.
    command_tx: mpsc::Sender<BlockchainManagerCommand>,
    /// The blocks sent to the blockchain manager.
    known_blocks: KnownBlocks,
}

impl BlockchainManagerHandle {
    /// Create a new handle and command receiver pair.
    pub(crate) fn new() -> (Self, mpsc::Receiver<BlockchainManagerCommand>) {
        let (command_tx, command_rx) = mpsc::channel(3);
        (
            Self {
                command_tx,
                known_blocks: KnownBlocks::default(),
            },
            command_rx,
        )
    }

    /// Returns the blocks sent to the blockchain manager.
    pub(crate) const fn known_blocks(&self) -> &KnownBlocks {
        &self.known_blocks
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
            return Err(IncomingBlockError::TooManyTxs);
        }

        let height = block_height(block.header.previous, blockchain_read_handle)
            .await?
            .ok_or(IncomingBlockError::Orphan)?
            + 1;

        let block_hash = block.hash();

        if block_height(block_hash, blockchain_read_handle)
            .await?
            .is_some()
        {
            return Ok(IncomingBlockOk::AlreadyHave);
        }

        // If this block is being handled, or is known to be invalid, then we can stop.
        if let Some(res) = self.known_blocks.response(&block_hash) {
            return res;
        }

        let TxpoolReadResponse::TxsForBlock { mut txs, missing } = txpool_read_handle
            .ready()
            .await?
            .call(TxpoolReadRequest::TxsForBlock(block.transactions.clone()))
            .await?
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
                    new_tx_verification_data(tx).map_err(BlockVerificationError::invalid_pow)?,
                );
            }
        }

        // Add the blocks hash to the blocks being handled.
        let outcome = match self
            .known_blocks
            .insert(block_hash, height, &block.serialize())
        {
            Ok(outcome) => outcome,
            // If another place is already adding this block, or found it invalid, then we can stop.
            Err(res) => return res,
        };

        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(BlockchainManagerCommand::AddBlock {
                block,
                prepped_txs: txs,
                response_tx,
                outcome,
            })
            .await
            .map_err(|_| IncomingBlockError::ChannelClosed)?;

        response_rx
            .await
            .map_err(|_| IncomingBlockError::ChannelClosed)?
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

/// Returns the height of the block with the given hash, if we have it.
async fn block_height(
    block_hash: [u8; 32],
    blockchain_read_handle: &mut BlockchainReadHandle,
) -> Result<Option<usize>, BlockchainError> {
    let BlockchainResponse::FindBlock(chain) = blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::FindBlock(block_hash))
        .await?
    else {
        unreachable!();
    };

    Ok(chain.map(|(_, height)| height))
}
