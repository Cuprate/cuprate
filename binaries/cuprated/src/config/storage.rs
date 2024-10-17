use serde::{Deserialize, Serialize};

use cuprate_database::config::SyncMode;
use cuprate_database_service::ReaderThreads;
use cuprate_helper::fs::{CUPRATE_BLOCKCHAIN_DIR, CUPRATE_TXPOOL_DIR};

/// The storage config.
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct StorageConfig {
    /// The amount of reader threads to spawn between the tx-pool and blockchain.
    pub reader_threads: ReaderThreads,
    /// The tx-pool config.
    pub txpool: TxpoolConfig,
    /// The blockchain config.
    pub blockchain: BlockchainConfig,
}

/// The blockchain config.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct BlockchainConfig {
    #[serde(flatten)]
    pub shared: SharedStorageConfig,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            shared: SharedStorageConfig {
                path: CUPRATE_BLOCKCHAIN_DIR.to_path_buf(),
                sync_mode: SyncMode::Async,
            },
        }
    }
}

/// The tx-pool config.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct TxpoolConfig {
    #[serde(flatten)]
    pub shared: SharedStorageConfig,

    /// The maximum size of the tx-pool (bytes).
    pub max_txpool_size: usize,
}

impl Default for TxpoolConfig {
    fn default() -> Self {
        Self {
            shared: SharedStorageConfig {
                path: CUPRATE_TXPOOL_DIR.to_path_buf(),
                sync_mode: SyncMode::Async,
            },
            max_txpool_size: 100_000_000,
        }
    }
}

/// Config values shared between the tx-pool and blockchain.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SharedStorageConfig {
    /// The path to the database storage.
    pub path: std::path::PathBuf,
    /// The [`SyncMode`] of the database.
    #[serde(default)]
    pub sync_mode: SyncMode,
}
