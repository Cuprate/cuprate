use std::path::PathBuf;

use super::macros::config_struct;
use crate::config::default::DefaultOrCustom;
use cuprate_blockchain::config::CacheSizes;
use cuprate_helper::fs::CUPRATE_DATA_DIR;
use serde::{Deserialize, Serialize};

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

        /// Test
        pub fjall_cache_size: DefaultOrCustom<u64>,

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
            reader_threads: cuprate_helper::thread::threads().get() * 4,
            fjall_cache_size: DefaultOrCustom::Default,
            txpool: Default::default(),
            blockchain: Default::default(),
        }
    }
}

config_struct! {
    /// The tx-pool config.
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TxpoolConfig {
        /// The maximum size of the tx-pool.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 100_000_000, 50_000_000
        pub max_txpool_byte_size: usize,

        /// The maximum age of transactions in the pool in seconds.
        /// Transactions will be dropped after this time is reached.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 100_000_000, 50_000_000
        pub maximum_age_secs: u64,
    }
}

config_struct! {
    /// The blockchain config.
    #[derive(Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct BlockchainConfig {
        /// Test
        pub tapes_cache_sizes: CacheSizes,
    }

}

impl Default for TxpoolConfig {
    fn default() -> Self {
        Self {
            max_txpool_byte_size: 100_000_000,
            maximum_age_secs: 60 * 60 * 24,
        }
    }
}
