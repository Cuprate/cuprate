mod cli;
mod config;
mod constants;
mod docs;
mod macros;

fn main() {
	// Handle CLI arguments.
	let (dry_run, log, config_cmd) = {
		// Nothing was passed, only `./cuprate`,
		// use the default values.
		if std::env::args_os().len() == 1 {
			(false, None, None)
		} else {
			// Else, run all the `clap` code (cli.rs).
			crate::cli::Cli::get()
		}
	};

	// Set `umask` for the entire process.
	// Files are `rw-r-x---`, folders are `rwx-r-x---`
	// https://docs.rs/disk/latest/disk/fn.umask.html
	//
	// TODO: set a reasonable value. Also, this does nothing on Windows.
	disk::umask(0o027);

	#[allow(non_snake_case)] // This is a reference to a `static` defined in `crate::config`.
	// Merge CLI options with on-disk config and init the logger.
	//
	// INVARIANT1: Logger gets set here, don't init elsewhere.
	// INVARIANT2: Initialize `CONFIG` - this must be set once only
	//
	// The reason the logger gets initialized here is because:
	// 1. We want to log within `init()`, but...
	// 2. We can't be sure what the true `--log-level` is
	//    until both CLI + disk Config are merged
	let CONFIG: &'static crate::config::Config = crate::config::ConfigBuilder::init(log, config_cmd);

	// If `dry_run`, print config/stats/etc and exit cleanly.
	if dry_run {
		println!("{}", serde_json::to_string_pretty(CONFIG).unwrap());
		std::process::exit(0);
	}

	// Cleanup cache files (if any).
	todo!("cuprate node .. cleaning cache");

	// Start `cuprate` (node init, other thread, TBD).
	let result = todo!("cuprate node ... init"); // <----------------- Program hangs here as the "node"

	// Graceful shutdown.
	// - Cleanup cache
	// - Flush any data
	// - Log some messages
	// - Wait on live connections
	//
	// CTRL+C triggers early return out of the above function,
	// another one after that will force an exit and force out of the below.
	match result {
		Ok(_) => todo!("cuprate node ... done"),
		Err(e) => todo!("cuprate node ... done, error {e}"),
	}
}