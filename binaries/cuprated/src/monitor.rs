use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::info;

/// A handle for task spawning and shutdown coordination.
#[derive(Clone)]
pub struct TaskExecutor {
    token: CancellationToken,
    tracker: TaskTracker,
    failed: Arc<AtomicBool>,
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
            failed: Arc::new(AtomicBool::new(false)),
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
    pub fn spawn_critical<F>(&self, future: F) -> JoinHandle<()>
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let executor = self.clone();
        self.tracker.spawn(async move {
            if let Err(e) = future.await {
                tracing::error!("{e:#}");
                executor.failed.store(true, Ordering::Relaxed);
            }
            executor.trigger_shutdown();
        })
    }

    /// Get the cancellation token.
    pub const fn cancellation_token(&self) -> &CancellationToken {
        &self.token
    }

    /// Returns true if any critical task failed.
    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
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
        tokio::select! {
            biased;
            () = task_executor.cancellation_token().cancelled() => {}
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
