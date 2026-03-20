//! `cuprated` CLI binary.
//!
//! Wrapper around [`cuprated::Node::launch`] that handles argument parsing,
//! logging setup, and the interactive command listener.

#![allow(
    unused_imports,
    unreachable_pub,
    unreachable_code,
    unused_crate_dependencies,
    dead_code,
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::unused_async,
    clippy::diverging_sub_expression,
    unused_mut,
    clippy::let_unit_value,
    clippy::needless_pass_by_ref_mut,
    reason = "TODO: remove after v1.0.0"
)]

use std::io::{self, IsTerminal};
use std::{thread::sleep, time::Duration};

use clap::Parser;
use tracing::info;

use cuprated::{
    commands::CommandHandle,
    config::{find_config, Args, Config},
    constants::{DEFAULT_CONFIG_STARTUP_DELAY, DEFAULT_CONFIG_WARNING},
    logging::eprintln_red,
};

fn main() {
    // Set global private permissions for created files.
    cuprate_helper::fs::set_private_global_file_permissions();

    // Initialize global static `LazyLock` data.
    cuprated::statics::init_lazylock_statics();

    // Parse CLI args and read config.
    let args = Args::parse();
    args.do_quick_requests();

    let mut config = load_config(&args);

    // Resolve `target_max_memory` from system RAM.
    if config.target_max_memory_is_default() {
        let mut info = sysinfo::System::new();
        info.refresh_memory();
        let memory = info.total_memory();
        if memory == 0 {
            eprintln_red("Unable to read total memory, please manually set the `target_max_memory` value in the config file.");
            std::process::exit(1);
        }
        config.set_target_max_memory_bytes(memory);
    }

    // Initialize logging.
    cuprated::logging::init_logging(&config);

    //Printing configuration
    info!("{config}");

    // Initialize thread pools.
    init_global_rayon_pool(&config);
    let rt = init_tokio_rt(&config);

    rt.block_on(async move {
        // Start the node.
        let cuprated::Node { command, .. } = cuprated::Node::launch(config).await;

        // If STDIN is a terminal, spawn a blocking thread for user input.
        if io::stdin().is_terminal() {
            spawn_stdin_reader(command);
        } else {
            // If no STDIN, await OS exit signal.
            info!("Terminal/TTY not detected, disabling STDIN commands");
        }

        tokio::signal::ctrl_c().await.unwrap();
    });
}

/// Spawn a STDIN reader that forwards commands to the [`CommandHandle`].
fn spawn_stdin_reader(command: CommandHandle) {
    let rt = tokio::runtime::Handle::current();

    std::thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match io::stdin().read_line(&mut line) {
                Ok(0) => return,
                Err(e) => {
                    eprintln!("Failed to read from stdin: {e}");
                    sleep(Duration::from_secs(1));
                    continue;
                }
                Ok(_) => {}
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match rt.block_on(command.send_command(trimmed.to_string())) {
                Ok(output) => println!("{output}"),
                Err(e) => {
                    eprintln!("Failed to send command: {e}");
                    return;
                }
            }
        }
    });
}

/// Load config: explicit path from `--config-file`, auto-detect from default
/// locations, or fall back to defaults with a warning.
fn load_config(args: &Args) -> Config {
    let config = if let Some(config_file) = &args.config_file {
        Config::read_from_path(config_file).unwrap_or_else(|e| {
            eprintln_red(&format!("Failed to read config from file: {e}"));
            std::process::exit(1);
        })
    } else if let Some(config) = find_config().unwrap_or_else(|e| {
        eprintln_red(&format!("Failed to read config: {e}"));
        std::process::exit(1);
    }) {
        config
    } else {
        if !args.skip_config_warning {
            eprintln_red(DEFAULT_CONFIG_WARNING);
            sleep(DEFAULT_CONFIG_STARTUP_DELAY);
        }
        Config::default()
    };

    let config = args.apply_args(config);

    if args.dry_run {
        let results = config.dry_run_check();
        let mut has_error = false;

        for check in &results {
            match &check.result {
                Ok(()) => println!("{}", check.description),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    has_error = true;
                }
            }
        }

        if has_error {
            eprintln_red("Checks failed.");
            std::process::exit(1);
        }

        println!("All checks passed successfully!");
        std::process::exit(0);
    }

    config
}

/// Initialize the global [`rayon`] thread-pool.
fn init_global_rayon_pool(config: &Config) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.rayon.threads)
        .thread_name(|index| format!("cuprated-rayon-{index}"))
        .build_global()
        .unwrap();
}

/// Initialize the [`tokio`] runtime.
fn init_tokio_rt(config: &Config) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.tokio.threads)
        .thread_name("cuprated-tokio")
        .enable_all()
        .build()
        .unwrap()
}
