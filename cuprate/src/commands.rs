//! Cuprate Subcommands

mod gernerate;
mod start;

use self::gernerate::GenerateCmd;
use self::start::StartCmd;
use crate::config::CuprateConfig;
use abscissa_core::{config::Override, Command, Configurable, FrameworkError, Runnable};
use std::path::PathBuf;

/// Cuprate Configuration Filename
pub const CONFIG_FILE: &str = "cuprate.toml";

/// Cuprate commands.
/// This is an enum unifying the main `Start` command from the other sub commands.
/// This allows us to separate the `Start` command from the other commands elsewhere.
#[derive(Runnable, Debug)]
pub enum CuprateCmd {
    Start(StartCmd),
    SubCmd(CuprateSubCmd),
}

/// Cuprate Subcommands
/// These commands run different helper functions separate from the main `start` command.
#[derive(clap::Parser, Command, Debug, Runnable, Clone)]
pub enum CuprateSubCmd {
    /// Generate a default config.
    Generate(GenerateCmd),
}

/// Entry point for the application.
#[derive(clap::Parser, Command, Debug)]
#[command(author, about, version)]
#[command(about =
"Welcome to Cuprate, the first and only alternative Monero node.",
long_about = None)]
pub struct EntryPoint {
    #[command(subcommand)]
    sub_cmd: Option<CuprateSubCmd>,

    /// The command thats used when no subcommand is specified, this
    /// command starts the node.
    ///
    /// This allows us to do `cuprate` instead of `cuprate start`
    #[clap(flatten)]
    pub start_args: StartCmd,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Use the specified config file
    #[arg(short, long)]
    pub config: Option<String>,
}

impl EntryPoint {
    fn cmd(&self) -> CuprateCmd {
        match &self.sub_cmd {
            Some(sub_cmd) => CuprateCmd::SubCmd(sub_cmd.clone()),
            None => CuprateCmd::Start(self.start_args.clone()),
        }
    }
}

impl Runnable for EntryPoint {
    fn run(&self) {
        self.cmd().run()
    }
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<CuprateConfig> for EntryPoint {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        // Check if the config file exists, and if it does not, ignore it.
        // If you'd like for a missing configuration file to be a hard error
        // instead, always return `Some(CONFIG_FILE)` here.
        let filename = self
            .config
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| CONFIG_FILE.into());

        if filename.exists() {
            Some(filename)
        } else {
            None
        }
    }

    /// Apply changes to the config after it's been loaded, e.g. overriding
    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(&self, config: CuprateConfig) -> Result<CuprateConfig, FrameworkError> {
        match &self.cmd() {
            CuprateCmd::Start(cmd) => cmd.override_config(config),
            _ => Ok(config)
            //
            // If you don't need special overrides for some
            // subcommands, you can just use a catch all
            // _ => Ok(config),
        }
    }
}
