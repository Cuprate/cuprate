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
        /// The fast data directory.
        ///
        /// This directory stores any long-term data where it is beneficial for fast access. This will
        /// be the same as the `slow_data_directory` unless changed.
        ///
        /// The default directories for each OS:
        ///
        /// | OS      | Path                                                |
        /// |---------|-----------------------------------------------------|
        /// | Windows | "C:\Users\Alice\AppData\Roaming\Cuprate\"           |
        /// | macOS   | "/Users/Alice/Library/Application Support/Cuprate/" |
        /// | Linux   | "/home/alice/.local/share/cuprate/"                 |
        pub fast_data_directory: PathBuf,

        #[comment_out = true]
        /// The slow data directory.
        ///
        /// This directory stores any long-term data where it is not that beneficial for fast access. This will
        /// be the same as the `fast_data_directory` unless changed.
        ///
        /// The default directories for each OS:
        ///
        /// | OS      | Path                                                |
        /// |---------|-----------------------------------------------------|
        /// | Windows | "C:\Users\Alice\AppData\Roaming\Cuprate\"           |
        /// | macOS   | "/Users/Alice/Library/Application Support/Cuprate/" |
        /// | Linux   | "/home/alice/.local/share/cuprate/"                 |
        pub slow_data_directory: PathBuf,

        #[comment_out = true]
        /// The cache directory.
        ///
        /// This directory store cache files.
        /// Although not recommended, this directory can be
        /// deleted without major disruption to cuprated.
        ///
        /// The default directories for each OS:
        ///
        /// | OS      | Path                                    |
        /// |---------|-----------------------------------------|
        /// | Windows | "C:\Users\Alice\AppData\Local\Cuprate\" |
        /// | macOS   | "/Users/Alice/Library/Caches/Cuprate/"  |
        /// | Linux   | "/home/alice/.cache/cuprate/"           |
        pub cache_directory: PathBuf,
    }
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            fast_data_directory: CUPRATE_DATA_DIR.to_path_buf(),
            slow_data_directory: CUPRATE_DATA_DIR.to_path_buf(),
            cache_directory: CUPRATE_CACHE_DIR.to_path_buf(),
        }
    }
}
