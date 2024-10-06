use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct StorageConfig {
    pub blockchain: cuprate_blockchain::config::Config,
    pub txpool: cuprate_txpool::Config,
}
