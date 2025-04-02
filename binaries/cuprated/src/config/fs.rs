use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cuprate_helper::fs::{CUPRATE_CACHE_DIR, CUPRATE_DATA_DIR};

use super::macros::config_struct;

config_struct! {
    /// The file system config.
    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct FileSystemConfig {
        #[comment_out = true]
        /// The data directory.
        pub data_directory: PathBuf,

        #[comment_out = true]
        /// The cache directory.
        pub cache_directory: PathBuf,
    }
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            data_directory: CUPRATE_DATA_DIR.to_path_buf(),
            cache_directory: CUPRATE_CACHE_DIR.to_path_buf(),
        }
    }
}
