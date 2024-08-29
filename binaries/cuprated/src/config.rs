//! cuprated config

use cuprate_blockchain::config::{
    Config as BlockchainConfig, ConfigBuilder as BlockchainConfigBuilder,
};

pub fn config() -> CupratedConfig {
    // TODO: read config options from the conf files & cli args.

    CupratedConfig {}
}

pub struct CupratedConfig {
    // TODO: expose config options we want to allow changing.
}

impl CupratedConfig {
    pub fn blockchain_config(&self) -> BlockchainConfig {
        BlockchainConfigBuilder::new().fast().build()
    }
}
