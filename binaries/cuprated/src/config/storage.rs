use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cuprate_database::config::SyncMode;
use cuprate_database_service::ReaderThreads;
use cuprate_fs::CUPRATE_DATA_DIR;

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
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 1, 16, 10
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
    Shared {
        #[comment_out = true]
        /// The sync mode of the database.
        ///
        /// Using "Safe" makes the DB less likely to corrupt
        /// if there is an unexpected crash, although it will
        /// make DB writes much slower.
        ///
        /// Valid values | "Fast", "Safe"
        pub sync_mode: SyncMode,
    }

    /// The blockchain config.
    #[derive(Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct BlockchainConfig { }

    /// The tx-pool config.
    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TxpoolConfig {
        /// The maximum size of the tx-pool.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 100_000_000, 50_000_000
        pub max_txpool_byte_size: usize,
    }
}

impl Default for TxpoolConfig {
    fn default() -> Self {
        Self {
            sync_mode: SyncMode::default(),
            max_txpool_byte_size: 100_000_000,
        }
    }
}
