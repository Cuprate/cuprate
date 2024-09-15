use std::collections::HashMap;

use monero_serai::block::Block;
use tokio::sync::oneshot;

use cuprate_types::TransactionVerificationData;

pub enum BlockchainManagerCommand {
    AddBlock {
        block: Block,
        prepped_txs: HashMap<[u8; 32], TransactionVerificationData>,
        response_tx: oneshot::Sender<Result<bool, anyhow::Error>>,
    },

    PopBlocks,
}
