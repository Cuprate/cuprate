use serde::{Deserialize, Serialize};

/// [`tokio`] config.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TokioConfig {
    /// The amount of threads to spawn for the async thread-pool
    pub threads: usize,
}

impl Default for TokioConfig {
    fn default() -> Self {
        Self {
            threads: cuprate_helper::thread::threads_75().get(),
        }
    }
}
