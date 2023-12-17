//---------------------------------------------------------------------------------------------------- Use
use clap::{Args, Parser, Subcommand};
use const_format::formatcp;
use disk::{Bincode2, Json, Plain, Toml};
use std::{
	num::NonZeroUsize,
	process::exit,
	net::IpAddr,
	path::PathBuf,
};
use serde::{Serialize, Deserialize};
use strum::{
    AsRefStr,
    Display,
    EnumCount,
    EnumIter,
    EnumString,
    EnumVariantNames,
    IntoStaticStr,
};

use crate::{
	config::{
		Config,
		ConfigBuilder,
		DEFAULT_LOG_LEVEL,
	},
	constants::{
		CUPRATE_BUILD_INFO,
		CUPRATE_COPYRIGHT,
		CUPRATE_CONFIG,
		CUPRATE_BIN,
	},
	docs::Docs,
};

//---------------------------------------------------------------------------------------------------- Constants
// Platform-specific binary name of `cuprate`.
const BIN: &str = formatcp!("{CUPRATE_BIN}{}", std::env::consts::EXE_SUFFIX);

// Appears at the top of `-h` and `--help`.
const USAGE: &str = formatcp!(
r#"{BIN} [OPTIONS] [COMMAND + OPTIONS] [ARGS...]

Arguments passed to `cuprate` will always take
priority over configuration options read from disk."#);

//---------------------------------------------------------------------------------------------------- CLI Parser (clap)
// `struct` encompassing all possible CLI argument values.
//
// This gets called by `main()` once, at the very beginning and is responsible for:
// - parsing/validating input values
// - routing certain `--flags` to function paths (and exiting)
// - possibly handing `CliResult` back off to `main()` for continued execution

#[derive(Parser)]
// Clap puts a really ugly non-wrapping list
// of possible args if this isn't set.
#[command(override_usage = USAGE)]
pub struct Cli {
	// Represents non-dashed subcommands, e.g: `./cuprate rpc`.
	#[command(subcommand)]
	sub_command: Option<SubCommand>,

	//------------------------------------------------------------------------------ Networking
	// TODO:
	// - add_peer
	// - add_priority_node
	// - add_exclusive_node
	// - ban_list
	// - no_upnp (no_idg)
	// - in_peers
	// - out_peers
	// - limit_rate
	// - limit_rate_up
	// - limit_rate_down
	// - max_connections_per_ip
	// - restricted_rpc
	// - rpc_ssl <true|false|auto>)
	// - rpc_login)
	// - bootstrap_daemon_address
	// - max_concurrency
	//
	// SOMEDAY:
	// - ZMQ (zmq_pub, no_zmq)
	// - Tor/i2p (proxy, tx_proxy, anonymous-inbound, pad_transactions)

	#[arg(long, verbatim_doc_comment)]
	/// The IPv4/IPv6 address `cuprate` will bind to for P2P [default: 0.0.0.0]
	p2p_ip: Option<IpAddr>,

	#[arg(long, verbatim_doc_comment)]
	/// The IPv4/IPv6 address `cuprate` will bind to for RPC [default: 127.0.0.1]
	rpc_ip: Option<IpAddr>,

	#[arg(long, verbatim_doc_comment)]
	/// The IPv4/IPv6 address `cuprate` will bind to for restricted RPC [default: 0.0.0.0]
	rpc_restricted_ip: Option<IpAddr>,

	#[arg(long, verbatim_doc_comment)]
	/// The port `cuprate` will bind to for P2P [default: 18080]
	///
	/// Using port `0` will select a random port.
	p2p_port: Option<u16>,

	#[arg(long, verbatim_doc_comment)]
	/// The port `cuprate` will bind to RPC [default: 18081]
	///
	/// Using port `0` will select a random port.
	rpc_port: Option<u16>,

	#[arg(long, verbatim_doc_comment)]
	/// The port `cuprate` will bind to RPC [default: 18089]
	///
	/// Using port `0` will select a random port.
	rpc_restricted_port: Option<u16>,

	#[arg(long, value_name = "OFF|ERROR|INFO|WARN|DEBUG|TRACE")]
	/// Set filter level for console logs
	log_level: Option<tracing::Level>,

	//------------------------------------------------------------------------------ Early Return
	// These are flags that do something
	// then immediately return, e.g `--docs`.
	//
	// Regardless of other flags provided, these will force a return.

	#[arg(long, verbatim_doc_comment)]
	/// Print the configuration `cuprate` would have used, but don't actually startup
	///
	/// This will go through the regular process of:
	///   - Reading disk for config
	///   - Reading command-line
	///   - Merging options together
	///   - Validating options
	///
	/// and then print them out as TOML, and exit.
	dry_run: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Open documentation locally in browser
	///
	/// This opens `cuprate'`s documentation in a web
	/// browser, and does not start `cuprate` itself.
	docs: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Print the PATHs used by `cuprate`
	///
	/// All data saved by `cuprate` is saved in these directories.
	/// For more information, see: <https://TODO>
	path: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Reset the current `cuprate.toml` config file to the default
	///
	/// Exits with `0` if everything went okay, otherwise shows error.
	reset_config: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Reset the `cuprate` cache folder
	reset_cache: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Delete all `cuprate` files that are on disk
	///
	/// This deletes all `daemon` Cuprate folders.
	/// The PATHs deleted will be printed on success.
	delete: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Print the default Cuprate config file
	print_config: bool,

	#[arg(long, verbatim_doc_comment)]
	/// Print all the JSON-RPC methods available
	print_methods: bool,

	#[arg(short, long)]
	/// Print version
	version: bool,
}

//---------------------------------------------------------------------------------------------------- Subcommands
// These are the enumerated "subcommands" for `cuprate`, e.g:
// ```
// # Subcommand
// ./cuprate rpc get_info
//
// # Flag
// ./cuprate --help
// ```
//
// Although, subcommands themselves can have their own `--flags`.
//
// Unlike `monerod`, RPC must be specified with the subcommand
// `./cuprate rpc get_info`. This isn't for any reason, it's just
// because setting up `./cuprate get_info` with Clap isn't obvious.
//
// Other `--flags` will still be parsed, e.g `./cuprate --config PATH rpc get_info` works

#[derive(Subcommand)]
pub enum SubCommand {
	#[command(verbatim_doc_comment)]
	/// TODO: below isn't accurate, we must take into account
	/// CLI and disk config before sending an RPC request
	///
	/// Send a JSON-RPC signal to a `cuprate` running on the same machine
	///
	/// This will not start a new `cuprate`, but send a
	/// signal to an already running one. This only works
	/// if there's a `cuprate` already running on the
	/// same machine.
	// Rpc(Rpc),
	TODO,
}

#[derive(Subcommand,Clone,Debug,Serialize,Deserialize)]
#[derive(AsRefStr,Display,EnumCount,EnumVariantNames,IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[command(rename_all = "snake_case")]
pub enum Rpc {
	// TODO: all available JSON-RPC methods.
	// probably defined outside of this crate.
	TODO,
}

//---------------------------------------------------------------------------------------------------- CLI argument handling
// Result of parsing the CLI arguments, to be passed back to `main()`.
pub struct CliResult {
	pub dry_run: bool,
	pub log_level: Option<tracing::Level>,
	pub config: Option<ConfigBuilder>,
}

impl CliResult {
	pub const DEFAULT: Self = Self {
		dry_run: false,
		log_level: Some(tracing::Level::INFO),
		config: None,
	};
}

impl Default for CliResult {
	fn default() -> Self {
		Self::DEFAULT
	}
}

//---------------------------------------------------------------------------------------------------- CLI argument handling
impl Cli {
	// `main()` calls this once.
	pub fn init() -> CliResult {
		Self::parse().handle_args()
	}

	// Handle all the values, routing code, and exiting early if needed.
	//
	// The order of the `if`'s are the precedence of the `--flags`'s
	// themselves, e.g `--version` will execute over all else.
	fn handle_args(mut self) -> CliResult {
		// TODO:
		// Calling `exit()` on each branch could
		// be replaced with something better,
		// although exit codes must be maintained.

		//-------------------------------------------------- Version.
		if self.version {
			println!("{CUPRATE_BUILD_INFO}\n{CUPRATE_COPYRIGHT}");
			exit(0);
		}

		//-------------------------------------------------- Path.
		if self.path {
			// Cache.
			let p: PathBuf = todo!();
			println!("{}", p.display());

			// Config.
			let p: PathBuf = todo!();
			println!("{}", p.display());

			#[cfg(not(target_os = "macos"))]
			{
				// `.local/share`
				let p: PathBuf = todo!();
				println!("{}", p.display());
			}

			exit(0);
		}

		//-------------------------------------------------- `reset_config`
		if self.reset_config {
			let p = Config::absolute_path().unwrap();
			Config::mkdir().unwrap();
			std::fs::write(&p, CUPRATE_CONFIG).unwrap();
			exit(0);
		}

		//-------------------------------------------------- `reset_cache`
		if self.reset_cache {
			let p: PathBuf = todo!();
			match std::fs::remove_dir_all(&p) {
				Ok(_)  => { eprintln!("{}", p.display()); exit(0); },
				Err(e) => { eprintln!("cuprate: Reset Cache failed: {e}"); exit(1); },
			}
		}

		//-------------------------------------------------- Docs.
		if self.docs {
			// Create documentation.
			if let Err(e) = Docs::create_open() {
				eprintln!("cuprate: Could not create docs: {e}");
				exit(1);
			}

			exit(0);
		}

		//-------------------------------------------------- Delete.
		if self.delete {
			#[cfg(not(target_os = "macos"))]
			let paths = [
				// Cache.
				todo!(),
				// Config.
				Config::sub_dir_parent_path().unwrap(),
				// `.local/share`
				todo!(),
			];

			#[cfg(target_os = "macos")]
			let paths = [
				// Cache.
				todo!(),
				// Config.
				Config::sub_dir_parent_path().unwrap(),
			];

			let mut code = 0;

			for p in paths {
				if !p.exists() {
					println!("cuprate: PATH does not exist ... {}", p.display());
					continue;
				}

				// TODO:
				// Although `disk` already does this,
				// maybe do sanity checks on these PATHs
				// to make sure we aren't doing `rm -rf /`.

				match std::fs::remove_dir_all(&p) {
					Ok(_) => println!("{}", p.display()),
					Err(e) => {
						eprintln!("cuprate error: {} - {e}", p.display());
						code = 1;
					},
				}
			}

			exit(code);
		}

		//-------------------------------------------------- Print
		if self.print_config {
			println!("{CUPRATE_CONFIG}");
			exit(0);
		} else if self.print_methods {
			for method in [0/* TODO(hinto): add methods iter */] {
				println!("{method}");
			}
			exit(0);
		}

		//-------------------------------------------------- Subcommands
		self.handle_subcommand();

		//-------------------------------------------------- Return to `main()`
		CliResult {
			dry_run: self.dry_run,
			log_level: self.log_level,
			config: self.map_cli_to_config(),
		}
	}

	// SOMEDAY:
	// we might have more subcommands other than
	// `./cuprate rpc <...>` in the future, so we're
	// treating this in a generic way.
	fn handle_subcommand(&self) {
		if let Some(c) = &self.sub_command {
			// TODO(hinto):
			// match and redirect sub command
			// match c {
				// SubCommand::Rpc(rpc) => self.handle_rpc(rpc),
			// }
			exit(0);
		} else {
			return;
		}
	}

	pub fn handle_rpc(&self, rpc: &Rpc) -> String /* TODO(hinto): should be json_rpc::Rpc or something */ {
		fn handle<T>(result: Result<T, anyhow::Error>) {
			if let Err(e) = result {
				eprintln!("{BIN} error: {e}");
				exit(1);
			} else {
				exit(0);
			}
		}

		// TODO(hinto):
		// should be some RPC enum to pass back
		// to `main()` so it can execute it
		todo!()
	}

	// Map CLI values into a proper `ConfigBuilder`
	// to be mixed with the on-disk values.
	//
	// This converts all necessary types from Clap into `ConfigBuilder`.
	//
	// This is needed as `Config` sometimes uses different storages/types
	// than what Clap will accept, e.g `Vec` -> `BTreeSet`.
	//
	// Mostly 0-clone, we're taking directly from `self` when possible.
	pub fn map_cli_to_config(&mut self) -> Option<ConfigBuilder> {
		// Special-case conversions.
		fn some_vec_to_btreeset<T: Ord>(vec: Option<Vec<T>>) -> Option<std::collections::BTreeSet<T>> {
			vec.map(|v| v.into_iter().collect())
		}

		// let mut exclusive_ips: BTreeSet<String /* TODO RPC enum */> = some_vec_to_btreeset(self.exclusive_ip.take());

		let mut log_level = self.log_level.take();

		// Our `ConfigBuilder` that we're mixing with `Cli`
		// options and returning back to `main()` for merging.
		let mut cb = ConfigBuilder::default();
		let mut diff = false;

		// If our `Cli` has a `Some(config_option)`,
		// swap it with our `ConfigBuilder` (cb).
		macro_rules! if_some {
			($($command:expr => $config:expr),* $(,)?) => {
				$(
					if $command.is_some() {
						std::mem::swap(&mut $command, &mut $config);
						// This is mutating the in-scope `diff` variable.
						// We can't just do `self == cb` since `ConfigBuilder`
						// is not `PartialEq`.
						diff = true;
					} else {
						$config = None;
					}
				)*
			}
		}

		if_some! {
			// TODO(hinto): add all config options.
			log_level => cb.log_level,
		}

		// If nothing was taken, then
		// there is no need to merge.
		if diff {
			Some(cb)
		} else {
			None
		}
	}
}
