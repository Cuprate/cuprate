use std::{io::Write, path::PathBuf};

use clap::builder::TypedValueParser;

use cuprate_helper::network::Network;

use crate::config::{default::create_default_config_file, Config, DEFAULT_CONFIG_FILE_NAME};

/// Cuprate Args.
#[derive(clap::Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// The network to run on.
    #[arg(
        long,
        default_value_t = Network::Mainnet,
        value_parser = clap::builder::PossibleValuesParser::new(["mainnet", "testnet", "stagenet"])
            .map(|s| s.parse::<Network>().unwrap()),
    )]
    pub network: Network,
    /// The amount of outbound clear-net connections to maintain.
    #[arg(long)]
    pub outbound_connections: Option<usize>,
    /// The PATH of the `cuprated` config file.
    #[arg(long)]
    pub config_file: Option<PathBuf>,
    /// Generate a config file and place it in the given PATH.
    #[arg(long)]
    pub generate_config: Option<PathBuf>,
}

impl Args {
    /// Apply the [`Args`] to the given [`Config`].
    ///
    /// This may exit the program if a config value was set that requires an early exit.
    pub fn apply_args(&self, mut config: Config) -> Config {
        if let Some(config_folder) = self.generate_config.as_ref() {
            // This will create the config file and exit.
            create_default_config_file(config_folder)
        };

        config.network = self.network;

        if let Some(outbound_connections) = self.outbound_connections {
            config.p2p.clear_net.general.outbound_connections = outbound_connections;
        }

        config
    }
}
