use serde::{Deserialize, Serialize};

/// The [`rayon`] config.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct RayonConfig {
    /// The number of threads to use for the [`rayon::ThreadPool`].
    pub threads: usize,
}

impl Default for RayonConfig {
    fn default() -> Self {
        Self {
            threads: cuprate_helper::thread::threads_75().get(),
        }
    }
}
