use std::{future::Future, panic::AssertUnwindSafe};

use futures::FutureExt;
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{error, info};

use crate::constants::CRITICAL_SERVICE_ERROR;

/// A handle for task spawning and shutdown coordination.
#[derive(Clone)]
pub struct TaskExecutor {
    token: CancellationToken,
    tracker: TaskTracker,
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskExecutor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            tracker: TaskTracker::new(),
        }
    }

    /// Spawn a tracked task.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn(future)
    }

    /// Spawn a critical tracked task that triggers shutdown on completion.
    ///
    /// Panics in the future are treated as errors.
    pub fn spawn_critical<F>(&self, name: &'static str, future: F) -> JoinHandle<()>
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let executor = self.clone();
        self.tracker.spawn(async move {
            match AssertUnwindSafe(future).catch_unwind().await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!(subsystem = name, "{e:#}"),
                Err(payload) => {
                    let msg = payload
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<no panic message>");
                    error!(subsystem = name, err = msg, "{CRITICAL_SERVICE_ERROR}");
                }
            }
            executor.trigger_shutdown();
        })
    }

    /// Get a clone of the cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Trigger a graceful shutdown.
    pub fn trigger_shutdown(&self) {
        if !self.token.is_cancelled() {
            info!("Shutting down...");
        }
        self.token.cancel();
    }

    /// Close the task tracker, preventing new tasks from being spawned.
    pub fn close(&self) {
        self.tracker.close();
    }

    /// Wait for all tracked tasks to complete.
    pub async fn wait(&self) {
        self.tracker.wait().await;
    }
}

/// Spawn a task that listens for OS signals and initiates shutdown.
pub fn spawn_signal_handler(task_executor: TaskExecutor) {
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
