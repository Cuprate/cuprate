use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cuprate_helper::fs::{CUPRATE_CACHE_DIR, CUPRATE_DATA_DIR};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FileSystemConfig {
    pub data_directory: PathBuf,
    pub cache_directory: PathBuf,
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            data_directory: CUPRATE_DATA_DIR.to_path_buf(),
            cache_directory: CUPRATE_CACHE_DIR.to_path_buf(),
        }
    }
}
