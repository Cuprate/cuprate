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
use tokio::sync::mpsc;
use tracing::info;

use cuprated::{
    config::{find_config, Args, Config},
    constants::{DEFAULT_CONFIG_STARTUP_DELAY, DEFAULT_CONFIG_WARNING},
    logging::eprintln_red,
};

fn main() {
    // Set global private permissions for created files.
    cuprate_helper::fs::set_private_global_file_permissions();

    // Parse CLI args and read config.
    let args = Args::parse();
    args.do_quick_requests();

    let config = load_config(&args);

    // Initialize logging.
    cuprated::logging::init_logging(&config);

    //Printing configuration
    info!("{config}");

    let rt = init_tokio_rt(&config);

    rt.block_on(async move {
        // Start the node.
        let cuprated::Node { command, .. } = cuprated::Node::launch(config).await;

        // Spawn a task to print command outputs received from the node.
        let mut output = command.output;
        tokio::spawn(async move {
            while let Some(msg) = output.recv().await {
                println!("{msg}");
            }
        });

        // If STDIN is a terminal, spawn a blocking thread for user input.
        if io::stdin().is_terminal() {
            stdin_loop(command.input).await;
        } else {
            // If no STDIN, await OS exit signal.
            info!("Terminal/TTY not detected, disabling STDIN commands");
            tokio::signal::ctrl_c().await.unwrap();
        }
    });
}

/// STDIN command listener loop.
async fn stdin_loop(command_tx: mpsc::Sender<String>) {
    let (tx, mut rx) = mpsc::channel::<String>(1);

    std::thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            if let Err(e) = stdin.read_line(&mut line) {
                eprintln!("Failed to read from stdin: {e}");
                sleep(Duration::from_secs(1));
                continue;
            }
            let trimmed = line.trim().to_string();
            if tx.blocking_send(trimmed).is_err() {
                return;
            }
        }
    });

    while let Some(line) = rx.recv().await {
        if !line.is_empty()
            && command_tx
                .send(line)
                .await
                .inspect_err(|err| eprintln!("Failed to send command: {err}"))
                .is_err()
        {
            break;
        }
    }
}

/// Load config: explicit path from `--config-file`, auto-detect from default
/// locations, or fall back to defaults with a warning.
fn load_config(args: &Args) -> Config {
    let config = if let Some(config_file) = &args.config_file {
        Config::read_from_path(config_file).unwrap_or_else(|e| {
            eprintln_red(&format!("Failed to read config from file: {e}"));
            std::process::exit(1);
        })
    } else if let Some(config) = find_config() {
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
        config.dry_run_check();
    }

    config
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
