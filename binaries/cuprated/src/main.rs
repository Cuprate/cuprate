//! `cuprated` CLI binary.
//!
//! Wrapper around [`cuprated::start`] that handles argument parsing,
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

use tokio::sync::mpsc;
use tracing::info;

mod commands;

fn main() {
    // Set global private permissions for created files.
    cuprate_helper::fs::set_private_global_file_permissions();

    let config = cuprated::config::read_config_and_args();

    // Initialize logging.
    cuprated::logging::init_logging(&config);

    //Printing configuration
    info!("{config}");

    let rt = init_tokio_rt(&config);

    rt.block_on(async move {
        // Start the node.
        let cuprated::Node { context_svc, .. } = cuprated::Node::launch(config).await;

        // Start the command listener.
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            let (command_tx, command_rx) = mpsc::channel(1);
            std::thread::spawn(|| commands::command_listener(command_tx));

            // Wait on the io_loop, spawned on a separate task as this improves performance.
            tokio::spawn(commands::io_loop(command_rx, context_svc))
                .await
                .unwrap();
        } else {
            // If no STDIN, await OS exit signal.
            info!("Terminal/TTY not detected, disabling STDIN commands");
            tokio::signal::ctrl_c().await.unwrap();
        }
    });
}

/// Initialize the [`tokio`] runtime.
fn init_tokio_rt(config: &cuprated::config::Config) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.tokio.threads)
        .thread_name("cuprated-tokio")
        .enable_all()
        .build()
        .unwrap()
}
