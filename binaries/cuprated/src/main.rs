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

use std::{
    io::{self, IsTerminal},
    process::ExitCode,
    thread::sleep,
    time::Duration,
};

use clap::Parser;
use tokio::sync::mpsc;
use tracing::info;

use cuprated::{
    config::{find_config, resolve_max_memory, Config},
    constants::{DEFAULT_CONFIG_STARTUP_DELAY, DEFAULT_CONFIG_WARNING},
    logging::eprintln_red,
    monitor::TaskExecutor,
};

mod args;
mod commands;

use crate::{
    args::Args,
    commands::{command_listener, Command},
};

fn main() -> ExitCode {
    match main_inner() {
        Ok(code) => code,
        Err(e) => {
            eprintln_red(&format!("{e:#}"));
            ExitCode::FAILURE
        }
    }
}

fn main_inner() -> anyhow::Result<ExitCode> {
    // Set global private permissions for created files.
    cuprate_helper::fs::set_private_global_file_permissions();

    // Parse CLI args and read config.
    let args = Args::parse();
    args.do_quick_requests();

    let mut config = load_config(&args)?;

    if args.dry_run {
        return Ok(dry_run_config(&config));
    }

    // Initialize logging.
    let _log_guard = cuprated::logging::init_logging(&config);

    // Resolve available memory.
    resolve_max_memory(&mut config)?;

    //Printing configuration
    info!("{config}");

    // Initialize the thread-pools
    init_global_rayon_pool(&config)?;

    let rt = init_tokio_rt(&config)?;

    #[expect(clippy::significant_drop_tightening)]
    let exit_code = rt.block_on(async move {
        // Start the node.
        let node = cuprated::Node::launch(config).await?;
        let task_executor = node.task_executor.clone();

        // Spawn OS signal handler.
        spawn_signal_handler(task_executor.clone());

        // If STDIN is a terminal, spawn a blocking thread for user input.
        if io::stdin().is_terminal() {
            let (command_tx, command_rx) = mpsc::channel(1);
            std::thread::spawn(|| commands::command_listener(command_tx));

            // Wait on the io_loop, spawned on a separate task as this improves performance.
            task_executor.spawn(commands::io_loop(command_rx, node));

            // Wait for shutdown and all tracked tasks to finish.
            task_executor.wait_for_shutdown().await?;
        } else {
            // If no STDIN, await shutdown signal.
            info!("Terminal/TTY not detected, disabling STDIN commands");
            node.wait_for_shutdown().await?;
        }

        anyhow::Ok(ExitCode::SUCCESS)
    })?;
    drop(rt);
    info!("Shutdown complete.");
    Ok(exit_code)
}

/// Spawn a task that listens for OS signals and initiates shutdown.
///
/// On the first signal, triggers a graceful shutdown via `task_executor`.
/// On the second signal, force-exits the process with code 1.
fn spawn_signal_handler(task_executor: TaskExecutor) {
    tokio::spawn(async move {
        let shutdown_token = task_executor.cancellation_token();
        tokio::select! {
            biased;
            () = shutdown_token.cancelled() => {}
            () = shutdown_signal() => {
                eprintln!();
                task_executor.trigger_shutdown();
                info!("Press Ctrl+C to force exit.");
            }
        }
        // Wait for second signal to force exit.
        shutdown_signal().await;
        eprintln!();
        std::process::exit(1);
    });
}

/// Wait for an OS shutdown signal (SIGINT or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install SIGINT handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

/// Load config: explicit path from `--config-file`, auto-detect from default
/// locations, or fall back to defaults with a warning.
fn load_config(args: &Args) -> anyhow::Result<Config> {
    let config = if let Some(config_file) = &args.config_file {
        Config::read_from_path(config_file)?
    } else if let Some(config) = find_config()? {
        config
    } else {
        if !args.skip_config_warning {
            eprintln_red(DEFAULT_CONFIG_WARNING);
            sleep(DEFAULT_CONFIG_STARTUP_DELAY);
        }
        Config::default()
    };

    Ok(args.apply_args(config))
}

/// Run the dry-run config checks.
fn dry_run_config(config: &Config) -> ExitCode {
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
        return ExitCode::FAILURE;
    }

    println!("All checks passed successfully!");
    ExitCode::SUCCESS
}

/// Initialize the [`tokio`] runtime.
fn init_tokio_rt(config: &Config) -> anyhow::Result<tokio::runtime::Runtime> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.tokio.threads)
        .thread_name("cuprated-tokio")
        .enable_all()
        .build()?;
    Ok(rt)
}

/// Initialize the global [`rayon`] thread-pool.
fn init_global_rayon_pool(config: &Config) -> anyhow::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.rayon.threads)
        .thread_name(|index| format!("cuprated-rayon-{index}"))
        .build_global()?;
    Ok(())
}
