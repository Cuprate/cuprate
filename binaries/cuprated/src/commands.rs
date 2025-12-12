//! Commands
//!
//! `cuprated` [`Command`] definition and handling.
use clap::{builder::TypedValueParser, Parser, ValueEnum};
use std::process::exit;
use std::{io, thread::sleep, time::Duration};
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
};
use cuprate_helper::time::secs_to_hms;

use crate::blockchain::handle::BlockchainManagerHandle;
use crate::monitor::CupratedMonitor;
use crate::{
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    logging::{self, CupratedTracingFilter},
    statics,
};

/// A command received from [`io::stdin`].
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
    PopBlocks {
        numb_blocks: usize,
    },

    Exit,
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

/// The [`Command`] listener loop.
pub fn command_listener(incoming_commands: mpsc::Sender<Command>) -> ! {
    let mut stdin = io::stdin();
    let mut line = String::new();

    loop {
        line.clear();

        if let Err(e) = stdin.read_line(&mut line) {
            eprintln!("Failed to read from stdin: {e}");
            sleep(Duration::from_secs(1));
            continue;
        }

        match Command::try_parse_from(line.split_whitespace()) {
            Ok(command) => drop(
                incoming_commands
                    .blocking_send(command)
                    .inspect_err(|err| eprintln!("Failed to send command: {err}")),
            ),
            Err(err) => err.print().unwrap(),
        }
    }
}

/// The [`Command`] handler loop.
pub async fn io_loop(
    mut incoming_commands: mpsc::Receiver<Command>,
    mut context_service: BlockchainContextService,
    mut blockchain_manager_handle: BlockchainManagerHandle,
    mut monitor: CupratedMonitor,
) {
    loop {
        let Some(command) = incoming_commands.recv().await else {
            tracing::warn!("Shutting down io_loop command channel closed.");
            return;
        };

        match command {
            Command::SetLog {
                level,
                output_target,
            } => {
                let modify_output = |filter: &mut CupratedTracingFilter| {
                    if let Some(level) = level {
                        filter.level = level;
                    }
                    println!("NEW LOG FILTER: {filter}");
                };

                match output_target {
                    OutputTarget::File => logging::modify_file_output(modify_output),
                    OutputTarget::Stdout => logging::modify_stdout_output(modify_output),
                }
            }
            Command::Status => {
                let context = context_service.blockchain_context();

                let uptime = statics::START_INSTANT.elapsed().unwrap_or_default();

                let (h, m, s) = secs_to_hms(uptime.as_secs());
                let height = context.chain_height;
                let top_hash = hex::encode(context.top_hash);

                println!("STATUS:\n  uptime: {h}h {m}m {s}s,\n  height: {height},\n  top_hash: {top_hash}");
            }
            Command::FastSyncStopHeight => {
                let stop_height = cuprate_fast_sync::fast_sync_stop_height();

                println!("{stop_height}");
            }
            Command::PopBlocks { numb_blocks } => {
                tracing::info!("Popping {numb_blocks} blocks.");
                let res = blockchain_manager_handle.pop_blocks(numb_blocks).await;

                match res {
                    Ok(()) => println!("Popped {numb_blocks} blocks."),
                    Err(e) => println!("Failed to pop blocks: {e}"),
                }
            }
            Command::Exit => {
                shutdown(monitor).await;
                return;
            }
        }
    }
}

async fn shutdown(mut monitor: CupratedMonitor) {
    tracing::info!("Exiting cuprated");
    monitor.cancellation_token.cancel();

    monitor.task_trackers.close();

    tracing::info!("Waiting for tasks to finish.");
    monitor.task_trackers.wait().await;
}
