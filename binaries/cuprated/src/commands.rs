use std::{io, thread::sleep, time::Duration};

use clap::{builder::TypedValueParser, Parser, ValueEnum};

use tokio::sync::mpsc;
use tracing::level_filters::LevelFilter;

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
