use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cuprate_database::config::SyncMode;
use cuprate_database_service::ReaderThreads;
use cuprate_helper::fs::CUPRATE_DATA_DIR;

/// The storage config.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct StorageConfig {
    /// The amount of reader threads to spawn between the tx-pool and blockchain.
    pub reader_threads: usize,
    /// The tx-pool config.
    pub txpool: TxpoolConfig,
    /// The blockchain config.
    pub blockchain: BlockchainConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            reader_threads: cuprate_helper::thread::threads_25().get(),
            txpool: Default::default(),
            blockchain: Default::default(),
        }
    }
}

/// The blockchain config.
#[derive(Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct BlockchainConfig {
    #[serde(flatten)]
    pub shared: SharedStorageConfig,
}

/// The tx-pool config.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
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
            shared: SharedStorageConfig::default(),
            max_txpool_byte_size: 100_000_000,
        }
    }
}

/// Config values shared between the tx-pool and blockchain.
#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct SharedStorageConfig {
    /// The [`SyncMode`] of the database.
    pub sync_mode: SyncMode,
}
