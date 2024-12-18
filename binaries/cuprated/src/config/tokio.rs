use serde::{Deserialize, Serialize};

/// [`tokio`] config.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct TokioConfig {
    /// The amount of threads to spawn for the async thread-pool
    pub threads: usize,
}

impl Default for TokioConfig {
    fn default() -> Self {
        Self {
            threads: (std::thread::available_parallelism().unwrap().get() * 3).div_ceil(4),
        }
    }
}
