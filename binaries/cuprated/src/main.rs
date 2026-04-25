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
use tracing::info;

use cuprated::{
    commands::CommandHandle,
    config::{find_config, Args, Config},
    constants::{DEFAULT_CONFIG_STARTUP_DELAY, DEFAULT_CONFIG_WARNING},
    logging::eprintln_red,
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
        return dry_run_config(&config);
    }

    // Resolve `target_max_memory` from system RAM.
    if config.target_max_memory_is_default() {
        let mut info = sysinfo::System::new();
        info.refresh_memory();
        let memory = info.total_memory();
        if memory == 0 {
            anyhow::bail!("Unable to read total memory, please manually set the `target_max_memory` value in the config file.");
        }
        config.set_target_max_memory_bytes(memory);
    }

    // Initialize logging.
    cuprated::logging::init_logging(&config);

    // Print configuration.
    info!("{config}");

    // Initialize the global rayon thread-pool.
    init_global_rayon_pool(&config)?;
    let rt = init_tokio_rt(&config)?;

    let exit_code = rt.block_on(async {
        // Start the node.
        let node = cuprated::Node::launch(config).await?;

        // Spawn OS signal handler.
        cuprated::monitor::spawn_signal_handler(node.task_executor.clone());

        // If STDIN is a terminal, spawn a blocking thread for user input.
        if io::stdin().is_terminal() {
            spawn_stdin_reader(node.command.clone());
        } else {
            info!("Terminal/TTY not detected, disabling STDIN commands");
        }

        match node.wait_for_shutdown().await {
            Ok(()) => Ok(ExitCode::SUCCESS),
            Err(e) => Err(e),
        }
    })?;
    drop(rt);
    info!("Shutdown complete.");
    Ok(exit_code)
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
fn dry_run_config(config: &Config) -> anyhow::Result<ExitCode> {
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
        anyhow::bail!("Checks failed.");
    }

    println!("All checks passed successfully!");
    Ok(ExitCode::SUCCESS)
}

/// Initialize the global [`rayon`] thread-pool.
fn init_global_rayon_pool(config: &Config) -> anyhow::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.rayon.threads)
        .thread_name(|index| format!("cuprated-rayon-{index}"))
        .build_global()?;
    Ok(())
}

/// Initialize the [`tokio`] runtime.
fn init_tokio_rt(config: &Config) -> anyhow::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.tokio.threads)
        .thread_name("cuprated-tokio")
        .enable_all()
        .build()?)
}
