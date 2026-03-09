//! Database configuration.
use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_helper::fs::CUPRATE_DATA_DIR;

/// The tapes cache sizes.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
pub struct CacheSizes {
    pub rct_outputs: u64,
    pub tx_infos: u64,
    pub block_infos: u64,
    pub pruned_blobs: u64,
    pub v1_prunable_blobs: u64,
    pub prunable_blobs: u64,
}

impl Default for CacheSizes {
    fn default() -> Self {
        Self {
            rct_outputs: 100 * 1024 * 1024,
            tx_infos: 1024 * 1024,
            block_infos: 1024 * 1024,
            pruned_blobs: 25 * 1024 * 1024,
            v1_prunable_blobs: 8 * 1024,
            prunable_blobs: 8 * 1024,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Config
/// The blockchain database configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// The directory where the blockchain blobs are stored.
    pub blob_dir: PathBuf,
    /// The directory where the blockchain indexes are stored.
    pub index_dir: PathBuf,
    /// The tapes cache sizes.
    pub cache_sizes: CacheSizes,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            blob_dir: CUPRATE_DATA_DIR.to_path_buf(),
            index_dir: CUPRATE_DATA_DIR.to_path_buf(),
            cache_sizes: CacheSizes::default(),
        }
    }
}
