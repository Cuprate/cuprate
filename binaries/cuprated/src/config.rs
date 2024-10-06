//! cuprated config
use serde::{Deserialize, Serialize};

mod sections;

use sections::P2PConfig;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    p2p: P2PConfig,
}
