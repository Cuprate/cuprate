//! Command line argument parsing and handling.

//---------------------------------------------------------------------------------------------------- Use
use std::path::PathBuf;

use clap::{crate_name, Args, Parser, Subcommand};

use crate::config::Config;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- CLI Parser (clap)
/// `struct` encompassing all possible CLI argument values.
///
/// This gets called by `main()` once, at the very beginning and is responsible for:
/// - parsing/validating input values
/// - routing certain `--flags` to function paths (and exiting)
/// - possibly handing `Config` back off to `main()` for continued execution
#[derive(Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    //------------------------------------------------------------------------------ TODO
    /// Set filter level for console logs.
    #[arg(
        long,
        value_name = "OFF|ERROR|INFO|WARN|DEBUG|TRACE",
        verbatim_doc_comment
    )]
    log_level: Option<String>, // FIXME: tracing::Level{Filter} doesn't work with clap?

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
    /// Print the PATHs used by `cuprate`
    ///
    /// All data saved by `cuprate` is saved in these directories.
    /// For more information, see: <https://TODO>
    path: bool,

    #[arg(long, verbatim_doc_comment)]
    /// Delete all `cuprate` files that are on disk
    ///
    /// This deletes all `daemon` Cuprate folders.
    /// The PATHs deleted will be printed on success.
    delete: bool,

    #[arg(short, long)]
    /// Print version and exit.
    version: bool,
}

//---------------------------------------------------------------------------------------------------- CLI default
impl Default for Cli {
    fn default() -> Self {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- CLI argument handling
impl Cli {
    /// `main()` calls this once.
    pub fn init() -> Self {
        match Self::parse().handle_args() {
            Ok(cli) => cli,
            Err(exit_code) => std::process::exit(exit_code),
        }
    }

    /// Handle all the values, routing code, and exiting early if needed.
    ///
    /// The order of the `if`'s are the precedence of the `--flags`'s
    /// themselves, e.g `--version` will execute over all else.
    fn handle_args(self) -> Result<Self, i32> {
        // TODO:
        // Calling `exit()` on each branch could
        // be replaced with something better,
        // although exit codes must be maintained.

        //-------------------------------------------------- Version.
        if self.version {
            println!("TODO");
            return Err(0);
        }

        //-------------------------------------------------- Path.
        if self.path {
            let p: PathBuf = todo!();
            println!("{}", p.display());
            return Err(0);
        }

        //-------------------------------------------------- Delete.
        if self.delete {
            let path = cuprate_helper::fs::cuprate_database_dir();

            if path.exists() {
                println!(
                    "{}: PATH does not exist '{}'",
                    crate_name!(),
                    path.display()
                );
                return Err(0);
            }

            match std::fs::remove_dir_all(path) {
                Ok(()) => {
                    println!("{}: deleted '{}'", crate_name!(), path.display());
                    return Err(0);
                }
                Err(e) => {
                    eprintln!("{} error: {} - {e}", crate_name!(), path.display());
                    return Err(1);
                }
            }
        }

        //-------------------------------------------------- Return `Config` to `main()`
        Ok(self)
    }
}
