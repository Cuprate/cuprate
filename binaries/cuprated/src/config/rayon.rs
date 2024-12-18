use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct RayonConfig {
    pub threads: usize,
}

impl Default for RayonConfig {
    fn default() -> Self {
        Self {
            threads: (std::thread::available_parallelism().unwrap().get() * 3).div_ceil(4),
        }
    }
}


