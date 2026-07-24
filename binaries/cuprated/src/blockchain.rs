//! Blockchain
//!
//! Contains the blockchain manager, syncer and an interface to mutate the blockchain.
use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::{mpsc, Notify};
use tower::{BoxError, Service, ServiceExt};

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{
    generate_genesis_block, BlockchainContext, BlockchainContextService, ContextConfig,
};
use cuprate_cryptonight::cryptonight_hash_v0;
use cuprate_p2p::{block_downloader::BlockDownloaderConfig, NetworkInterface};
use cuprate_p2p_core::{client::PeerSyncCallback, ClearNet, Network};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    VerifiedBlockInformation,
};

mod chain_service;
mod error;
mod fast_sync;
pub mod interface;
mod manager;
mod syncer;
mod types;

pub use error::{BlockManagerError, BlockValidationError, IncomingBlockError};
pub use fast_sync::get_fast_sync_hashes;
pub use interface::BlockchainManagerHandle;
pub use manager::IncomingBlockOk;
pub use syncer::BlockchainSyncerHandle;
pub use types::ConsensusBlockchainReadHandle;

pub(crate) use manager::init_blockchain_manager;

/// The interface to the blockchain.
#[derive(Clone)]
pub struct BlockchainInterface {
    /// A read handle to the blockchain database.
    read: BlockchainReadHandle,
    /// The blockchain context service.
    context_svc: BlockchainContextService,
    /// A handle to the blockchain manager.
    manager: BlockchainManagerHandle,
    /// A handle to the blockchain syncer.
    syncer: BlockchainSyncerHandle,
}

impl BlockchainInterface {
    pub(crate) const fn new(
        read: BlockchainReadHandle,
        context_svc: BlockchainContextService,
        manager: BlockchainManagerHandle,
        syncer: BlockchainSyncerHandle,
    ) -> Self {
        Self {
            read,
            context_svc,
            manager,
            syncer,
        }
    }

    /// Returns a read handle to the blockchain database.
    pub fn read(&self) -> BlockchainReadHandle {
        self.read.clone()
    }

    /// Returns the current [`BlockchainContext`].
    pub fn context(&mut self) -> &BlockchainContext {
        self.context_svc.blockchain_context()
    }

    /// Returns a handle to the blockchain manager.
    pub fn manager(&self) -> BlockchainManagerHandle {
        self.manager.clone()
    }

    /// Returns a handle to the blockchain syncer.
    pub fn syncer(&self) -> BlockchainSyncerHandle {
        self.syncer.clone()
    }

    /// Returns the blockchain context service.
    pub(crate) fn context_svc(&self) -> BlockchainContextService {
        self.context_svc.clone()
    }

    /// Creates a [`PeerSyncCallback`] that filters and wakes the syncer.
    pub(crate) fn peer_sync_callback(&self) -> PeerSyncCallback {
        self.syncer
            .callback(self.context_svc.clone(), self.manager.clone())
    }
}

/// Checks if the genesis block is in the blockchain and adds it if not.
pub async fn check_add_genesis(
    blockchain_read_handle: &mut BlockchainReadHandle,
    blockchain_write_handle: &mut BlockchainWriteHandle,
    network: Network,
) -> anyhow::Result<()> {
    // Try to get the chain height, will fail if the genesis block is not in the DB.
    if blockchain_read_handle
        .ready()
        .await?
        .call(BlockchainReadRequest::ChainHeight)
        .await
        .is_ok()
    {
        return Ok(());
    }

    let genesis = generate_genesis_block(network);

    assert_eq!(genesis.miner_transaction().prefix().outputs.len(), 1);
    assert!(genesis.transactions.is_empty());

    blockchain_write_handle
        .ready()
        .await?
        .call(BlockchainWriteRequest::WriteBlock(
            VerifiedBlockInformation {
                block_blob: genesis.serialize(),
                txs: vec![],
                block_hash: genesis.hash(),
                pow_hash: cryptonight_hash_v0(&genesis.serialize_pow_hash()),
                height: 0,
                generated_coins: genesis.miner_transaction().prefix().outputs[0]
                    .amount
                    .unwrap(),
                weight: genesis.miner_transaction().weight(),
                long_term_weight: genesis.miner_transaction().weight(),
                cumulative_difficulty: 1,
                block: genesis,
            },
        ))
        .await?;

    Ok(())
}

/// Initializes the consensus services.
pub async fn init_consensus(
    blockchain_read_handle: BlockchainReadHandle,
    context_config: ContextConfig,
) -> Result<BlockchainContextService, BoxError> {
    let read_handle = ConsensusBlockchainReadHandle::new(blockchain_read_handle, BoxError::from);

    let ctx_service =
        cuprate_consensus::initialize_blockchain_context(context_config, read_handle.clone())
            .await?;

    Ok(ctx_service)
}
