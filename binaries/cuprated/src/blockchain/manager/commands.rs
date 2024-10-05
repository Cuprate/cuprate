//! This module contains the commands for the blockchain manager.
use std::collections::HashMap;

use monero_serai::block::Block;
use tokio::sync::oneshot;

use cuprate_types::TransactionVerificationData;

/// The blockchain manager commands.
pub enum BlockchainManagerCommand {
    /// Attempt to add a new block to the blockchain.
    AddBlock {
        /// The [`Block`] to add.
        block: Block,
        /// All the transactions defined in [`Block::transactions`].
        prepped_txs: HashMap<[u8; 32], TransactionVerificationData>,
        /// The channel to send the response down.
        response_tx: oneshot::Sender<Result<IncomingBlockOk, anyhow::Error>>,
    },
}

/// The [`Ok`] response for an incoming block.
pub enum IncomingBlockOk {
    /// The block was added to the main-chain.
    AddedToMainChain,
    /// The blockchain manager is not ready yet.
    NotReady,
    /// The block was added to an alt-chain.
    AddedToAltChain,
    /// We already have the block.
    AlreadyHave,
}
