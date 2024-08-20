//! Block batch handling functions.

use crate::blockchain::types::ConsensusBlockchainReadHandle;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::context::NewBlockData;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
    BlockVerifierService, BlockchainReadRequest, BlockchainResponse, ExtendedConsensusError,
    VerifyBlockRequest, VerifyBlockResponse, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_p2p::block_downloader::BlockBatch;
use cuprate_types::blockchain::BlockchainWriteRequest;
use cuprate_types::{Chain, HardFork};
use tower::{Service, ServiceExt};
use tracing::{debug, error, info};

pub async fn handle_incoming_block_batch<C, TxV>(
    batch: BlockBatch,
    block_verifier_service: &mut BlockVerifierService<C, TxV, ConsensusBlockchainReadHandle>,
    blockchain_context_service: &mut C,
    blockchain_read_handle: &mut BlockchainReadHandle,
    blockchain_write_handle: &mut BlockchainWriteHandle,
) where
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
    let (first_block, _) = batch
        .blocks
        .first()
        .expect("Block batch should not be empty");

    match blockchain_read_handle
        .oneshot(BlockchainReadRequest::FindBlock(
            first_block.header.previous,
        ))
        .await
    {
        Err(_) | Ok(BlockchainResponse::FindBlock(None)) => {
            // The block downloader shouldn't be downloading orphan blocks
            error!("Failed to find parent block for first block in batch.");
            return;
        }
        Ok(BlockchainResponse::FindBlock(Some((chain, _)))) => match chain {
            Chain::Main => {
                handle_incoming_block_batch_main_chain(
                    batch,
                    block_verifier_service,
                    blockchain_context_service,
                    blockchain_write_handle,
                )
                .await;
            }
            Chain::Alt(_) => todo!(),
        },

        Ok(_) => panic!("Blockchain service returned incorrect response"),
    }
}

async fn handle_incoming_block_batch_main_chain<C, TxV>(
    batch: BlockBatch,
    block_verifier_service: &mut BlockVerifierService<C, TxV, ConsensusBlockchainReadHandle>,
    blockchain_context_service: &mut C,
    blockchain_write_handle: &mut BlockchainWriteHandle,
) where
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
    let Ok(VerifyBlockResponse::MainChainBatchPrepped(prepped)) = block_verifier_service
        .ready()
        .await
        .expect("TODO")
        .call(VerifyBlockRequest::MainChainBatchPrepareBlocks {
            blocks: batch.blocks,
        })
        .await
    else {
        info!("Error verifying batch, banning peer");
        todo!()
    };

    for (block, txs) in prepped {
        let Ok(VerifyBlockResponse::MainChain(verified_block)) = block_verifier_service
            .ready()
            .await
            .expect("TODO")
            .call(VerifyBlockRequest::MainChainPrepped { block, txs })
            .await
        else {
            info!("Error verifying batch, banning peer");
            todo!()
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
