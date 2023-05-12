//! `start` subcommand.

/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;

use crate::config::CuprateConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};

/// `start` subcommand
///
/// This is the main daemon entry point.
#[derive(clap::Parser, Command, Debug, Default, Clone)]
pub struct StartCmd {
    /// Remove when we have more commandss.
    #[clap(help_heading = Some("Options Used Only On Start"))]
    #[arg(short, long)]
    t: bool,
}

impl Runnable for StartCmd {
    /// Start the application.
    fn run(&self) {
        let config = APP.config();
        let r = APP.authors();
        println!("Hello, {}!", &config.hello.recipient);
        println!("{r:?}");
    }
}

impl config::Override<CuprateConfig> for StartCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(&self, mut config: CuprateConfig) -> Result<CuprateConfig, FrameworkError> {
        Ok(config)
    }
}
