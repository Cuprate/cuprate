use clap::{builder::TypedValueParser, Parser};
use std::io;
use std::io::Stdin;
use std::iter::once;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::level_filters::LevelFilter;

// strip out usage
const PARSER_TEMPLATE: &str = "\
        {all-args}
    ";
// strip out name/version
const APPLET_TEMPLATE: &str = "\
        {about-with-newline}\n\
        {all-args}\
    ";

#[derive(Debug, Parser)]
#[command(multicall = true, subcommand_required = true, rename_all = "snake_case", help_template = PARSER_TEMPLATE, arg_required_else_help = true, disable_help_flag = true)]
pub enum Command {
    /// Change the log output.
    #[command(arg_required_else_help = true, help_template = APPLET_TEMPLATE)]
    SetLog {
        /// The minimum log level that will be displayed.
        #[arg(
          short, long,
          value_parser = clap::builder::PossibleValuesParser::new(["off", "trace", "debug", "info", "warn", "error"])
            .map(|s| s.parse::<LevelFilter>().unwrap()),
        )]
        level: LevelFilter,
    },
    /// Print status information on `cuprated`.
    #[command(help_template = APPLET_TEMPLATE)]
    Status,
}

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
