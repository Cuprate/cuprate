use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_helper::cast::usize_to_u64;
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use cuprate_types::Chain;
use monero_serai::block::Block;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};
use tower::{Service, ServiceExt};
use crate::blockchain::manager::commands::BlockchainManagerCommand;

pub static INCOMING_BLOCK_TX: OnceLock<mpsc::Sender<BlockchainManagerCommand>> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    #[error("Unknown transactions in block.")]
    UnknownTransactions(Vec<u64>),
    #[error("The block has an unknown parent.")]
    Orphan,
    #[error(transparent)]
    InvalidBlock(anyhow::Error),
}

pub async fn handle_incoming_block(
    block: Block,
    given_txs: Vec<Transaction>,
    blockchain_read_handle: &mut BlockchainReadHandle,
) -> Result<bool, IncomingBlockError> {
    if !block_exists(block.header.previous, blockchain_read_handle).await.expect("TODO") {
        return Err(IncomingBlockError::Orphan);
    }

    let block_hash = block.hash();

    if block_exists(block_hash, blockchain_read_handle)
        .await
        .expect("TODO")
    {
        return Ok(false);
    }

    // TODO: Get transactions from the tx pool first.
    if given_txs.len() != block.transactions.len() {
        return Err(IncomingBlockError::UnknownTransactions(
            (0..usize_to_u64(block.transactions.len())).collect(),
        ));
    }

    let prepped_txs = given_txs
        .into_par_iter()
        .map(|tx| {
            let tx = new_tx_verification_data(tx)?;
            Ok((tx.tx_hash, tx))
        })
        .collect::<Result<_, anyhow::Error>>()
        .map_err(IncomingBlockError::InvalidBlock)?;

    let Some(incoming_block_tx) = INCOMING_BLOCK_TX.get() else {
        return Ok(false);
    };

    let (response_tx, response_rx) = oneshot::channel();

    incoming_block_tx
        .send(  BlockchainManagerCommand::AddBlock {
            block,
            prepped_txs,
            response_tx,
        })
        .await
        .expect("TODO: don't actually panic here");

    response_rx.await.unwrap().map_err(IncomingBlockError::InvalidBlock)
}

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
