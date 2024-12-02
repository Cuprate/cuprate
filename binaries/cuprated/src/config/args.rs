use std::{io::Write, path::PathBuf, process::exit};

use clap::builder::TypedValueParser;

use cuprate_helper::network::Network;

use crate::{config::Config, constants::EXAMPLE_CONFIG};

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
    /// Generate a config file and print it to stdout.
    #[arg(long)]
    pub generate_config: bool,
}

impl Args {
    /// Complete any quick requests asked for in [`Args`].
    ///
    /// May cause the process to [`exit`].
    pub fn do_quick_requests(&self) {
        if self.generate_config {
            println!("{EXAMPLE_CONFIG}");
            exit(0);
        };
    }

    /// Apply the [`Args`] to the given [`Config`].
    ///
    /// This may exit the program if a config value was set that requires an early exit.
    pub const fn apply_args(&self, mut config: Config) -> Config {
        config.network = self.network;

        if let Some(outbound_connections) = self.outbound_connections {
            config.p2p.clear_net.general.outbound_connections = outbound_connections;
        }

        config
    }
}
