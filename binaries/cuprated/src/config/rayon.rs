use serde::{Deserialize, Serialize};

use super::macros::config_struct;

config_struct! {
    /// The [`rayon`] config.
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct RayonConfig {
        #[comment_out = true]
        /// Type         | Number
        /// Valid values | >= 1
        /// Examples     | 1, 8, 14
        pub threads: usize,
    }
}

impl Default for RayonConfig {
    fn default() -> Self {
        Self {
            threads: cuprate_helper::thread::threads().get(),
        }
    }
}
