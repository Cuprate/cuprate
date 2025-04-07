use serde::{Deserialize, Serialize};

use super::macros::config_struct;

config_struct! {
    /// [`tokio`] config.
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TokioConfig {
        #[comment_out = true]
        /// The amount of threads to spawn for the tokio thread-pool.
        ///
        /// Type         | Number
        /// Valid values | >= 1
        /// Examples     | 1, 8, 14
        pub threads: usize,
    }
}

impl Default for TokioConfig {
    fn default() -> Self {
        Self {
            threads: cuprate_helper::thread::threads_75().get(),
        }
    }
}
