use std::time::Duration;

use tokio::sync::watch;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::info;

/// Owned by `main`. Used to wait for all tracked tasks after cancellation.
pub struct CupratedMonitor {
    pub task_tracker: TaskTracker,
    pub cancellation_token: CancellationToken,
    pub error_watch: watch::Receiver<bool>,
}

/// A cloneable handle passed to tasks for spawning sub-tasks.
#[derive(Clone)]
pub struct CupratedTask {
    pub task_tracker: TaskTracker,
    pub cancellation_token: CancellationToken,
    pub error_set: watch::Sender<bool>,
}

/// Create a new [`CupratedMonitor`] and [`CupratedTask`] pair.
///
/// Must be called exactly once at startup.
pub fn new() -> (CupratedMonitor, CupratedTask) {
    let (error_set, error_watch) = watch::channel(false);
    let task_tracker = TaskTracker::new();
    let cancellation_token = CancellationToken::new();

    (
        CupratedMonitor {
            task_tracker: task_tracker.clone(),
            cancellation_token: cancellation_token.clone(),
            error_watch,
        },
        CupratedTask {
            task_tracker,
            cancellation_token,
            error_set,
        },
    )
}

/// Trigger a graceful shutdown.
pub fn trigger_shutdown(token: &CancellationToken) {
    info!("Shutting down gracefully... Press Ctrl+C again to exit immediately.");
    token.cancel();
}

/// Spawn a task that listens for OS signals and initiates shutdown.
pub fn spawn_signal_handler(token: CancellationToken) {
    tokio::spawn(async move {
        shutdown_signal().await;
        trigger_shutdown(&token);
        shutdown_signal().await;
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
