//! Command line argument parsing and handling.

//---------------------------------------------------------------------------------------------------- Use
use std::path::PathBuf;

use clap::{crate_name, Args, Parser, Subcommand};

use cuprate_helper::fs::cuprate_database_dir;

use crate::config::Config;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- CLI Parser (clap)
/// `struct` encompassing all possible CLI argument values.
///
/// This gets called by `main()` once, at the very beginning and is responsible for:
/// - parsing/validating input values
/// - routing certain `--flags` to function paths (and exiting)
/// - possibly handing `Config` back off to `main()` for continued execution
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    //------------------------------------------------------------------------------ Config options
    /// Set filter level for console logs.
    #[arg(
        long,
        value_name = "OFF|ERROR|INFO|WARN|DEBUG|TRACE",
        verbatim_doc_comment
    )]
    pub(crate) log_level: Option<String>, // FIXME: tracing::Level{Filter} doesn't work with clap?

    /// TODO
    #[arg(long, value_name = "PATH", verbatim_doc_comment)]
    pub(crate) config: Option<PathBuf>,

    /// TODO
    #[arg(long, value_name = "PATH", verbatim_doc_comment)]
    pub(crate) db_config: Option<PathBuf>,

    //------------------------------------------------------------------------------ Early Return
    // These are flags that do something then immediately return.
    //
    // Regardless of other flags provided, these will force a return.
    #[arg(long, verbatim_doc_comment)]
    /// Print the configuration `cuprate-database-benchmark`
    /// would have used, but don't actually startup.
    ///
    /// This will go through the regular process of:
    ///   - Reading command-line
    ///   - Reading disk for config
    ///
    /// and then print the config out as TOML, and exit.
    pub(crate) dry_run: bool,

    #[arg(long, verbatim_doc_comment)]
    /// Print the PATH used by `cuprate-database`.
    pub(crate) path: bool,

    #[arg(long, verbatim_doc_comment)]
    /// Delete all `cuprated-database` files that are on disk.
    ///
    /// This deletes the PATH returned by `--path`, aka, the
    /// directory that stores all database related files.
    pub(crate) delete: bool,

    #[arg(short, long)]
    /// Print various stats and exit.
    pub(crate) version: bool,
}

//---------------------------------------------------------------------------------------------------- CLI default
// impl Default for Cli {
//     fn default() -> Self {
//         Self {
//             log_level: None,
//             db_config: None,
//             dry_run: false,
//             path: false,
//             delete: false,
//             version: false,
//         }
//     }
// }

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
            todo!();
            return Err(0);
        }

        //-------------------------------------------------- Path.
        if self.path {
            let path = cuprate_database_dir();
            println!("{}", path.display());
            return Err(0);
        }

        //-------------------------------------------------- Delete.
        if self.delete {
            let path = cuprate_database_dir();

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
