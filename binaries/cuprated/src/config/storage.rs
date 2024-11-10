use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cuprate_database::config::SyncMode;
use cuprate_database_service::ReaderThreads;
use cuprate_helper::fs::CUPRATE_DATA_DIR;

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

    /// The maximum size of the tx-pool.
    pub max_txpool_byte_size: usize,
}

impl Default for TxpoolConfig {
    fn default() -> Self {
        Self {
            shared: SharedStorageConfig {
                sync_mode: SyncMode::Async,
            },
            max_txpool_byte_size: 100_000_000,
        }
    }
}

/// Config values shared between the tx-pool and blockchain.
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct SharedStorageConfig {
    /// The [`SyncMode`] of the database.
    pub sync_mode: SyncMode,
}
