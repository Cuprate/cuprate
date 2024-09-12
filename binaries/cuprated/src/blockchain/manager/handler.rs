use crate::blockchain::types::ConsensusBlockchainReadHandle;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockVerifierService,
    ExtendedConsensusError, VerifyBlockRequest, VerifyBlockResponse, VerifyTxRequest,
    VerifyTxResponse,
};
use cuprate_p2p::block_downloader::BlockBatch;
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};
use cuprate_types::AltBlockInformation;
use monero_serai::block::Block;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use tower::{Service, ServiceExt};

async fn handle_incoming_alt_block<C, TxV>(
    block: Block,
    txs: Vec<Transaction>,
    current_cumulative_difficulty: u128,
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
    let prepared_txs = txs
        .into_par_iter()
        .map(|tx| {
            let tx = new_tx_verification_data(tx)?;
            (tx.tx_hash, tx)
        })
        .collect::<Result<_, _>>()?;

    let VerifyBlockResponse::AltChain(alt_block_info) = block_verifier_service
        .ready()
        .await
        .expect("TODO")
        .call(VerifyBlockRequest::AltChain {
            block,
            prepared_txs,
        })
        .await?
    else {
        panic!("Incorrect response!");
    };

    if alt_block_info.cumulative_difficulty > current_cumulative_difficulty {
        todo!("do re-org");
    }

    blockchain_write_handle
        .ready()
        .await
        .expect("TODO")
        .call(BlockchainWriteRequest::WriteAltBlock(alt_block_info))?;

    Ok(())
}

async fn try_do_reorg<C, TxV>(
    top_alt_block: AltBlockInformation,
    chain_height: usize,
    block_verifier_service: &mut BlockVerifierService<C, TxV, ConsensusBlockchainReadHandle>,
    blockchain_context_service: &mut C,
    blockchain_write_handle: &mut BlockchainWriteHandle,
    blockchain_read_handle: &mut BlockchainReadHandle,
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
    let BlockchainResponse::AltBlocksInChain(mut alt_blocks) = blockchain_read_handle
        .ready()
        .await
        .expect("TODO")
        .call(BlockchainReadRequest::AltBlocksInChain(
            top_alt_block.chain_id,
        ))
        .await?
    else {
        panic!("Incorrect response!");
    };

    alt_blocks.push(top_alt_block);

    let split_height = alt_blocks[0].height;

    let BlockchainResponse::PopBlocks(old_main_chain_id) = blockchain_write_handle
        .ready()
        .await
        .expect("TODO")
        .call(BlockchainWriteRequest::PopBlocks(
            chain_height - split_height + 1,
        ))
        .await?
    else {
        panic!("Incorrect response!");
    };

    todo!()
}

async fn verify_add_alt_blocks_to_main_chain<C, TxV>(
    alt_blocks: Vec<AltBlockInformation>,
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
    let VerifyBlockResponse::AltChain(alt_block_info) = block_verifier_service
        .ready()
        .await
        .expect("TODO")
        .call(VerifyBlockRequest::MainChainPrepped { block, txs })
        .await?
    else {
        panic!("Incorrect response!");
    };
}
