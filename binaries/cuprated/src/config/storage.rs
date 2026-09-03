use serde::{Deserialize, Serialize};

use cuprate_blockchain::config::{CacheSizes, Persistence};

use super::{default::DefaultOrCustom, macros::config_struct};

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

        #[comment_out = true]
        /// The size of the fjall read cache.
        ///
        /// Fjall recommends using 20 to 25 % of available memory.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 64_000_000
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
        /// Whether to prune the blockchain. This reduces the size of the blockchain by removing most transaction proof data.
        /// Note that once a node is pruned it cannot be un-pruned.
        ///
        /// Defaults to `false`.
        pub prune: bool,
        #[comment_out = true]
        /// The persistence mode of the database.
        ///
        /// ## Buffer
        /// Buffer the changes but don't wait for them to be synced to disk.
        /// This can lead to corruption if there is a crash.
        ///
        /// ## Sync
        /// Sync all changes to disk.
        /// This prevents corruption but can be a bit slower.
        ///
        /// ## BufferThenSync
        /// Buffer changes while syncing but then switch to syncing all changes to disk once synced.
        /// This is a compromise between Buffer and Sync.
        ///
        /// Type         | String
        /// Valid values | "Buffer", "Sync", "BufferThenSync"
        /// Examples     | "BufferThenSync"
        pub persistence: Persistence,

        #[inline = true]
        #[comment_out = true]
        /// The size of each tape cache.
        ///
        /// You probably do not need to edit these values.
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
