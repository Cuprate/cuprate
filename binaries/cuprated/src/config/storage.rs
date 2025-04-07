use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cuprate_database::config::SyncMode;
use cuprate_database_service::ReaderThreads;
use cuprate_helper::fs::CUPRATE_DATA_DIR;

use super::macros::config_struct;

config_struct! {
    /// The storage config.
    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct StorageConfig {
        #[comment_out = true]
        /// The amount of reader threads to spawn for the tx-pool and blockchain.
        ///
        /// The tx-pool and blockchain both share a single threadpool.
        pub reader_threads: usize,

        #[child = true]
        /// The tx-pool config.
        pub txpool: TxpoolConfig,

        #[child = true]
        /// The blockchain config.
        pub blockchain: BlockchainConfig,
    }
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

config_struct! {
    /// The blockchain config.
    #[derive(Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct BlockchainConfig {
        #[flatten = true]
        /// Shared config.
        ##[serde(flatten)]
        pub shared: SharedStorageConfig,
    }
}

config_struct! {
    /// The tx-pool config.
    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TxpoolConfig {
        #[flatten = true]
        /// Shared config.
        ##[serde(flatten)]
        pub shared: SharedStorageConfig,

        /// The maximum size of the tx-pool.
        pub max_txpool_byte_size: usize,
    }
}

impl Default for TxpoolConfig {
    fn default() -> Self {
        Self {
            shared: SharedStorageConfig::default(),
            max_txpool_byte_size: 100_000_000,
        }
    }
}

config_struct! {
    /// Config values shared between the tx-pool and blockchain.
    #[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct SharedStorageConfig {
        #[comment_out = true]
        /// The sync mode of the database.
        ///
        /// Changing this value could make the DB a lot slower when writing data, although
        /// using "Safe" makes the DB more durable if there was an unexpected crash.
        ///
        /// Valid values: ["Fast", "Safe"].
        pub sync_mode: SyncMode,
    }
}
