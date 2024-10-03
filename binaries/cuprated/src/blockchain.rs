//! Blockchain
//!
//! Will contain the chain manager and syncer.
use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::{mpsc, Notify};
use tower::{BoxError, Service, ServiceExt};

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{generate_genesis_block, BlockChainContextService, ContextConfig};
use cuprate_cryptonight::cryptonight_hash_v0;
use cuprate_p2p::{block_downloader::BlockDownloaderConfig, NetworkInterface};
use cuprate_p2p_core::{ClearNet, Network};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    VerifiedBlockInformation,
};

mod interface;
mod manager;
mod syncer;
mod types;

use types::{
    ConcreteBlockVerifierService, ConcreteTxVerifierService, ConsensusBlockchainReadHandle,
};

pub use interface::{handle_incoming_block, IncomingBlockError};

/// Checks if the genesis block is in the blockchain and adds it if not.
pub async fn check_add_genesis(
    blockchain_read_handle: &mut BlockchainReadHandle,
    blockchain_write_handle: &mut BlockchainWriteHandle,
    network: Network,
) {
    // Try to get the chain height, will fail if the genesis block is not in the DB.
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

    assert_eq!(genesis.miner_transaction.prefix().outputs.len(), 1);
    assert!(genesis.transactions.is_empty());

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

/// Initializes the consensus services.
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
    let read_handle = ConsensusBlockchainReadHandle::new(blockchain_read_handle, BoxError::from);

    let ctx_service =
        cuprate_consensus::initialize_blockchain_context(context_config, read_handle.clone())
            .await?;

    let (block_verifier_svc, tx_verifier_svc) =
        cuprate_consensus::initialize_verifier(read_handle, ctx_service.clone());

    Ok((block_verifier_svc, tx_verifier_svc, ctx_service))
}
