use std::{io, thread::sleep, time::Duration};

use clap::{builder::TypedValueParser, Parser, ValueEnum};
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
};
use cuprate_helper::time::secs_to_hms;

use crate::{
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    logging::{self, CupratedTracingFilter},
    statics,
};

const PARSER_TEMPLATE: &str = "{all-args}";

/// A command received from [`io::stdin`].
#[derive(Debug, Parser)]
#[command(
    multicall = true,
    subcommand_required = true,
    rename_all = "snake_case",
    help_template = PARSER_TEMPLATE,
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
        #[arg(value_enum, default_value_t = OutputTarget::Stdout)]
        output_target: OutputTarget,
    },
    /// Print status information on `cuprated`.
    Status,
}

/// The log output target.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputTarget {
    /// The stdout logging output.
    Stdout,
    /// The file appender logging output.
    File,
}

/// The [`Command`] listener loop.
pub fn command_listener(incoming_commands: mpsc::Sender<Command>) -> ! {
    let mut stdin = io::stdin();
    let mut line = String::new();

    loop {
        match stdin.read_line(&mut line) {
            Ok(_) => match Command::try_parse_from(line.trim().split(' ')) {
                Ok(command) => incoming_commands.blocking_send(command).unwrap(),
                Err(err) => err.print().unwrap(),
            },
            Err(e) => {
                println!("Failed to read from stdin: {e}");

                sleep(Duration::from_secs(1));
            }
        }

        line.clear();
    }
}

/// The [`Command`] handler loop.
pub async fn io_loop(
    mut incoming_commands: mpsc::Receiver<Command>,
    mut context_service: BlockChainContextService,
) -> ! {
    while let Some(command) = incoming_commands.recv().await {
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
                let BlockChainContextResponse::Context(blockchain_context) = context_service
                    .ready()
                    .await
                    .expect(PANIC_CRITICAL_SERVICE_ERROR)
                    .call(BlockChainContextRequest::Context)
                    .await
                    .expect(PANIC_CRITICAL_SERVICE_ERROR)
                else {
                    unreachable!();
                };
                let context = blockchain_context.unchecked_blockchain_context();

                let uptime = statics::START_INSTANT.elapsed().unwrap_or_default();
                let (hours, minutes, second) = secs_to_hms(uptime.as_secs());

                println!("STATUS:\n  uptime: {hours}h {minutes}m {second}s,\n  height: {},\n  top_hash: {}", context.chain_height, hex::encode(context.top_hash));
            }
        }
    }

    unreachable!()
}
