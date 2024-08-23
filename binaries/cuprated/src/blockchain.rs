//! Blockchain
//!
//! Will contain the chain manager and syncer.

use crate::blockchain::manager::BlockchainManager;
use crate::blockchain::types::{
    ChainService, ConcreteBlockVerifierService, ConcreteTxVerifierService,
    ConsensusBlockchainReadHandle,
};
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{generate_genesis_block, BlockChainContextService, ContextConfig};
use cuprate_cryptonight::cryptonight_hash_v0;
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{ClearNet, Network};
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};
use cuprate_types::VerifiedBlockInformation;
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};

mod manager;
mod syncer;
mod types;

pub async fn check_add_genesis(
    blockchain_read_handle: &mut BlockchainReadHandle,
    blockchain_write_handle: &mut BlockchainWriteHandle,
    network: &Network,
) {
    if blockchain_read_handle
        .ready()
        .await
        .unwrap()
        .call(BlockchainReadRequest::ChainHeight)
        .await
        .is_ok()
    {
        return;
    }

    let genesis = generate_genesis_block(network);

    blockchain_write_handle
        .ready()
        .await
        .unwrap()
        .call(BlockchainWriteRequest::WriteBlock(
            VerifiedBlockInformation {
                block_blob: genesis.serialize(),
                txs: vec![],
                block_hash: genesis.hash(),
                pow_hash: cryptonight_hash_v0(&genesis.serialize_pow_hash()),
                height: 0,
                generated_coins: genesis.miner_transaction.prefix().outputs[0]
                    .amount
                    .unwrap(),
                weight: genesis.miner_transaction.weight(),
                long_term_weight: genesis.miner_transaction.weight(),
                cumulative_difficulty: 1,
                block: genesis,
            },
        ))
        .await
        .unwrap();
}

pub async fn init_consensus(
    blockchain_read_handle: BlockchainReadHandle,
    context_config: ContextConfig,
) -> Result<
    (
        ConcreteBlockVerifierService,
        ConcreteTxVerifierService,
        BlockChainContextService,
    ),
    tower::BoxError,
> {
    let ctx_service = cuprate_consensus::initialize_blockchain_context(
        context_config,
        ConsensusBlockchainReadHandle(blockchain_read_handle.clone()),
    )
    .await?;

    let (block_verifier_svc, tx_verifier_svc) = cuprate_consensus::initialize_verifier(
        ConsensusBlockchainReadHandle(blockchain_read_handle),
        ctx_service.clone(),
    );

    Ok((block_verifier_svc, tx_verifier_svc, ctx_service))
}

pub fn init_blockchain_manager(
    clearnet_interface: NetworkInterface<ClearNet>,
    block_downloader_config: BlockDownloaderConfig,
    blockchain_write_handle: BlockchainWriteHandle,
    blockchain_read_handle: BlockchainReadHandle,
    blockchain_context_service: BlockChainContextService,
    block_verifier_service: ConcreteBlockVerifierService,
) {
    let (batch_tx, batch_rx) = mpsc::channel(1);

    tokio::spawn(syncer::syncer(
        blockchain_context_service.clone(),
        ChainService(blockchain_read_handle.clone()),
        clearnet_interface,
        batch_tx,
        block_downloader_config,
    ));

    let manager = BlockchainManager::new(
        blockchain_write_handle,
        blockchain_read_handle,
        blockchain_context_service,
        block_verifier_service,
    );

    tokio::spawn(manager.run(batch_rx));
}
