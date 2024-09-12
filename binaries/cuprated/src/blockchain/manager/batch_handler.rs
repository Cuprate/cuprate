//! Block batch handling functions.

use crate::blockchain::types::ConsensusBlockchainReadHandle;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::context::NewBlockData;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
    BlockVerifierService, BlockchainReadRequest, BlockchainResponse, ExtendedConsensusError,
    VerifyBlockRequest, VerifyBlockResponse, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_p2p::block_downloader::BlockBatch;
use cuprate_types::blockchain::BlockchainWriteRequest;
use cuprate_types::{Chain, HardFork};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::{debug, error, info};

async fn handle_incoming_block_batch_main_chain<C, TxV>(
    batch: BlockBatch,
    block_verifier_service: &mut BlockVerifierService<C, TxV, ConsensusBlockchainReadHandle>,
    blockchain_context_service: &mut C,
    blockchain_write_handle: &mut BlockchainWriteHandle,
) -> Result<(), anyhow::Error>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,

    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>
        + Clone
        + Send
        + 'static,
    TxV::Future: Send + 'static,
{
    info!(
        "Handling batch to main chain height: {}",
        batch.blocks.first().unwrap().0.number().unwrap()
    );

    let VerifyBlockResponse::MainChainBatchPrepped(prepped) = block_verifier_service
        .ready()
        .await
        .expect("TODO")
        .call(VerifyBlockRequest::MainChainBatchPrepareBlocks {
            blocks: batch.blocks,
        })
        .await?
    else {
        panic!("Incorrect response!");
    };

    for (block, txs) in prepped {
        let VerifyBlockResponse::MainChain(verified_block) = block_verifier_service
            .ready()
            .await
            .expect("TODO")
            .call(VerifyBlockRequest::MainChainPrepped { block, txs })
            .await?
        else {
            panic!("Incorrect response!");
        };

        blockchain_context_service
            .ready()
            .await
            .expect("TODO")
            .call(BlockChainContextRequest::Update(NewBlockData {
                block_hash: verified_block.block_hash,
                height: verified_block.height,
                timestamp: verified_block.block.header.timestamp,
                weight: verified_block.weight,
                long_term_weight: verified_block.long_term_weight,
                generated_coins: verified_block.generated_coins,
                vote: HardFork::from_vote(verified_block.block.header.hardfork_signal),
                cumulative_difficulty: verified_block.cumulative_difficulty,
            }))
            .await
            .expect("TODO");

        blockchain_write_handle
            .ready()
            .await
            .expect("TODO")
            .call(BlockchainWriteRequest::WriteBlock(verified_block))
            .await
            .expect("TODO");
    }
}

async fn handle_incoming_block_batch_alt_chain<C, TxV>(
    batch: BlockBatch,
    block_verifier_service: &mut BlockVerifierService<C, TxV, ConsensusBlockchainReadHandle>,
    blockchain_context_service: &mut C,
    blockchain_write_handle: &mut BlockchainWriteHandle,
) -> Result<(), anyhow::Error>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,

    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>
        + Clone
        + Send
        + 'static,
    TxV::Future: Send + 'static,
{
    for (block, txs) in batch.blocks {
        alt_block_info.cumulative_difficulty
    }
}
