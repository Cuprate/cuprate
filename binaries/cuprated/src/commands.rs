//! Commands
//!
//! `cuprated` command definition and handling.

use clap::{builder::TypedValueParser, Parser, ValueEnum};
use tracing::level_filters::LevelFilter;

use cuprate_consensus_context::BlockchainContextService;
use cuprate_helper::time::secs_to_hms;

use crate::{
    logging::{self, CupratedTracingFilter},
    statics,
};

/// A command received from user input.
#[derive(Debug, Parser)]
#[command(
    multicall = true,
    subcommand_required = true,
    rename_all = "snake_case",
    help_template = "{all-args}",
    arg_required_else_help = true,
    disable_help_flag = true
)]
pub enum Command {
    /// Change the log output.
    #[command(arg_required_else_help = true)]
    SetLog {
        /// The minimum log level that will be displayed.
        #[arg(
          short, long,
          value_parser = clap::builder::PossibleValuesParser::new(["off", "trace", "debug", "info", "warn", "error"])
            .map(|s| s.parse::<LevelFilter>().unwrap()),
        )]
        level: Option<LevelFilter>,
        /// The logging output target to change.
        #[arg(value_enum, default_value_t)]
        output_target: OutputTarget,
    },

    /// Print status information on `cuprated`.
    Status,

    /// Print the height of first block not contained in the fast sync hashes.
    FastSyncStopHeight,

    /// Pop blocks from the top of the blockchain.
    PopBlocks { numb_blocks: usize },
}

/// The log output target.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputTarget {
    /// The stdout logging output.
    #[default]
    Stdout,
    /// The file appender logging output.
    File,
}

/// Parse and execute a raw command string. Returns the output.
pub async fn handle_command(input: &str, context_service: &mut BlockchainContextService) -> String {
    let command = match Command::try_parse_from(input.split_whitespace()) {
        Ok(cmd) => cmd,
        Err(err) => return format!("{err}"),
    };

    match command {
        Command::SetLog {
            level,
            output_target,
        } => {
            let mut msg = String::new();
            let modify_output = |filter: &mut CupratedTracingFilter| {
                if let Some(level) = level {
                    filter.level = level;
                }
                msg = format!("NEW LOG FILTER: {filter}");
            };

            match output_target {
                OutputTarget::File => logging::modify_file_output(modify_output),
                OutputTarget::Stdout => logging::modify_stdout_output(modify_output),
            }

            msg
        }
        Command::Status => {
            let context = context_service.blockchain_context();

            let uptime = statics::START_INSTANT.elapsed().unwrap_or_default();

            let (h, m, s) = secs_to_hms(uptime.as_secs());
            let height = context.chain_height;
            let top_hash = hex::encode(context.top_hash);
            format!(
                "STATUS:\n  uptime: {h}h {m}m {s}s,\n  height: {height},\n  top_hash: {top_hash}"
            )
        }
        Command::FastSyncStopHeight => {
            let stop_height = cuprate_fast_sync::fast_sync_stop_height();

            format!("{stop_height}")
        }
        Command::PopBlocks { numb_blocks } => {
            tracing::info!("Popping {numb_blocks} blocks.");
            let res = crate::blockchain::interface::pop_blocks(numb_blocks).await;

            match res {
                Ok(()) => format!("Popped {numb_blocks} blocks."),
                Err(e) => format!("Failed to pop blocks: {e}"),
            }
        }
    }
}
