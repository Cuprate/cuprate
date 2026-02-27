use std::future::Future;

use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::info;

use crate::error::CupratedError;

/// Used to wait for all tracked tasks after cancellation.
pub struct CupratedSupervisor {
    pub task_tracker: TaskTracker,
    pub cancellation_token: CancellationToken,
}

/// A cloneable handle passed to tasks for spawning sub-tasks.
#[derive(Clone)]
pub struct CupratedTask {
    pub task_tracker: TaskTracker,
    pub cancellation_token: CancellationToken,
}

impl CupratedTask {
    /// Spawn a task whose failure triggers a graceful shutdown.
    pub fn spawn_critical<F>(&self, fut: F) -> JoinHandle<Result<(), CupratedError>>
    where
        F: Future<Output = Result<(), CupratedError>> + Send + 'static,
    {
        let token = self.cancellation_token.clone();
        self.task_tracker.spawn(async move {
            let result = fut.await;
            if let Err(ref e) = result {
                if !token.is_cancelled() {
                    tracing::error!("{e}");
                    trigger_shutdown(&token);
                }
            }
            result
        })
    }
}

/// Create a new [`CupratedSupervisor`] and [`CupratedTask`] pair.
///
/// Must be called exactly once at startup.
pub fn new() -> (CupratedSupervisor, CupratedTask) {
    let task_tracker = TaskTracker::new();
    let cancellation_token = CancellationToken::new();

    (
        CupratedSupervisor {
            task_tracker: task_tracker.clone(),
            cancellation_token: cancellation_token.clone(),
        },
        CupratedTask {
            task_tracker,
            cancellation_token,
        },
    )
}

/// Service error handler: suppress if shutting down, otherwise trigger shutdown and return error.
pub fn shutdown_or_err<T>(
    token: &CancellationToken,
    error: impl Into<anyhow::Error>,
    default: T,
) -> Result<T, anyhow::Error> {
    if token.is_cancelled() {
        Ok(default)
    } else {
        trigger_shutdown(token);
        Err(error.into())
    }
}

/// Trigger a graceful shutdown.
pub fn trigger_shutdown(token: &CancellationToken) {
    if !token.is_cancelled() {
        info!("Shutting down gracefully... Press Ctrl+C again to exit immediately.");
    }
    token.cancel();
}

/// Spawn a task that listens for OS signals and initiates shutdown.
pub fn spawn_signal_handler(token: CancellationToken) {
    tokio::spawn(async move {
        shutdown_signal().await;
        eprintln!();
        trigger_shutdown(&token);
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
