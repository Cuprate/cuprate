//! Command line argument parsing and handling.

//---------------------------------------------------------------------------------------------------- Use
use clap::{Args, Parser, Subcommand};

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
    // #[arg(long, value_name = "OFF|ERROR|INFO|WARN|DEBUG|TRACE")]
    // Set filter level for console logs
    // log_level: Option<tracing::Level>,

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

//---------------------------------------------------------------------------------------------------- CLI argument handling
impl Cli {
    /// `main()` calls this once.
    pub fn init() -> Config {
        // Self::parse().handle_args()
        todo!()
    }

    //     /// Handle all the values, routing code, and exiting early if needed.
    //     ///
    //     /// The order of the `if`'s are the precedence of the `--flags`'s
    //     /// themselves, e.g `--version` will execute over all else.
    //     fn handle_args(mut self) -> Config {
    //         // TODO:
    //         // Calling `exit()` on each branch could
    //         // be replaced with something better,
    //         // although exit codes must be maintained.

    //         //-------------------------------------------------- Version.
    //         if self.version {
    //             println!("{CUPRATE_BUILD_INFO}\n{CUPRATE_COPYRIGHT}");
    //             exit(0);
    //         }

    //         //-------------------------------------------------- Path.
    //         if self.path {
    //             // Cache.
    //             let p: PathBuf = todo!();
    //             println!("{}", p.display());

    //             // Config.
    //             let p: PathBuf = todo!();
    //             println!("{}", p.display());

    //             #[cfg(not(target_os = "macos"))]
    //             {
    //                 // `.local/share`
    //                 let p: PathBuf = todo!();
    //                 println!("{}", p.display());
    //             }

    //             exit(0);
    //         }

    //         //-------------------------------------------------- `reset_config`
    //         if self.reset_config {
    //             let p = Config::absolute_path().unwrap();
    //             Config::mkdir().unwrap();
    //             std::fs::write(&p, CUPRATE_CONFIG).unwrap();
    //             exit(0);
    //         }

    //         //-------------------------------------------------- `reset_cache`
    //         if self.reset_cache {
    //             let p: PathBuf = todo!();
    //             match std::fs::remove_dir_all(&p) {
    //                 Ok(_) => {
    //                     eprintln!("{}", p.display());
    //                     exit(0);
    //                 }
    //                 Err(e) => {
    //                     eprintln!("cuprate: Reset Cache failed: {e}");
    //                     exit(1);
    //                 }
    //             }
    //         }

    //         //-------------------------------------------------- Docs.
    //         if self.docs {
    //             // Create documentation.
    //             if let Err(e) = Docs::create_open() {
    //                 eprintln!("cuprate: Could not create docs: {e}");
    //                 exit(1);
    //             }

    //             exit(0);
    //         }

    //         //-------------------------------------------------- Delete.
    //         if self.delete {
    //             #[cfg(not(target_os = "macos"))]
    //             let paths = [
    //                 // Cache.
    //                 todo!(),
    //                 // Config.
    //                 Config::sub_dir_parent_path().unwrap(),
    //                 // `.local/share`
    //                 todo!(),
    //             ];

    //             #[cfg(target_os = "macos")]
    //             let paths = [
    //                 // Cache.
    //                 todo!(),
    //                 // Config.
    //                 Config::sub_dir_parent_path().unwrap(),
    //             ];

    //             let mut code = 0;

    //             for p in paths {
    //                 if !p.exists() {
    //                     println!("cuprate: PATH does not exist ... {}", p.display());
    //                     continue;
    //                 }

    //                 // TODO:
    //                 // Although `disk` already does this,
    //                 // maybe do sanity checks on these PATHs
    //                 // to make sure we aren't doing `rm -rf /`.

    //                 match std::fs::remove_dir_all(&p) {
    //                     Ok(_) => println!("{}", p.display()),
    //                     Err(e) => {
    //                         eprintln!("cuprate error: {} - {e}", p.display());
    //                         code = 1;
    //                     }
    //                 }
    //             }

    //             exit(code);
    //         }

    //         //-------------------------------------------------- Print
    //         if self.print_config {
    //             println!("{CUPRATE_CONFIG}");
    //             exit(0);
    //         } else if self.print_methods {
    //             for method in [0 /* TODO(hinto): add methods iter */] {
    //                 println!("{method}");
    //             }
    //             exit(0);
    //         }

    //         //-------------------------------------------------- Subcommands
    //         self.handle_subcommand();

    //         //-------------------------------------------------- Return to `main()`
    //         Config {
    //             dry_run: self.dry_run,
    //             log_level: self.log_level,
    //             config: self.map_cli_to_config(),
    //         }
    //     }
}
