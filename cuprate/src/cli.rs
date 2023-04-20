use clap::{Command, Arg, ArgAction, ArgMatches, value_parser, ArgGroup};
use tracing::{Span, Level, span, event};
use crate::CUPRATE_VERSION;

/// This function simply contains clap arguments
pub fn args() -> ArgMatches {
	Command::new("Cuprate")
		.version(CUPRATE_VERSION)
		.author("Cuprate's contributors")
		.about("An upcoming experimental, modern, and secure monero node")

		// Generic Arguments
		.arg(Arg::new("log")
				.long("log-level")
				.value_name("Level")
				.help("Set the log level")
				.value_parser(value_parser!(u8))
				.default_value("1")
				.long_help("Set the log level. There is 3 log level: <1~INFO, 2~DEBUG >3~TRACE.")
				.required(false)
				.action(ArgAction::Set)
			)
		.get_matches()
}

/// This function initialize the FmtSubscriber used by tracing to display event in the console. It send back a span used during runtime.
pub fn init(matches: &ArgMatches) -> Span {

	// Getting the log level from args
	let log_level = matches.get_one::<u8>("log").unwrap();
	let level_filter = match log_level {
		2 => Level::DEBUG,
		x if x > &2 => Level::TRACE,
		_ => Level::INFO,
	};

	// Initializing tracing subscriber and runtime span
	let subscriber = tracing_subscriber::FmtSubscriber::builder().with_max_level(level_filter).with_target(false).finish();
	tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber for tracing. We prefer to abort the node since without it you have no output in the console");
	let runtime_span = span!(Level::INFO, "Runtime");
	let _guard = runtime_span.enter();

	// Notifying log level
	event!(Level::INFO, "Log level set to {}", level_filter);

	drop(_guard);
	runtime_span
}