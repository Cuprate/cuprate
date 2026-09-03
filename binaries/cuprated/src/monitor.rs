//! Task spawning and shutdown coordination.

use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::FutureExt;
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{debug, error, info};

/// An unexpected node-side failure that should trigger a shutdown.
pub type FatalError = tower::BoxError;

/// Why the node stopped.
#[must_use]
pub enum ShutdownReason {
    /// Shutdown was requested.
    Requested,

    /// A critical task failed.
    TaskFailed,
}

/// A handle for task spawning and shutdown coordination.
#[derive(Clone, Default)]
pub struct TaskExecutor {
    token: CancellationToken,
    tracker: TaskTracker,
    failed: Arc<AtomicBool>,
}

impl TaskExecutor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a tracked task.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn(future)
    }

    /// Spawn a tracked task that triggers shutdown if the future returns early.
    pub fn spawn_critical<F>(&self, name: &'static str, future: F) -> JoinHandle<()>
    where
        F: Future<Output = Result<(), FatalError>> + Send + 'static,
    {
        let executor = self.clone();
        self.tracker.spawn(future.map(move |result| {
            if executor.token.is_cancelled() {
                // Node is shutting down, so an early exit or error is expected
                if let Err(e) = result {
                    debug!(subsystem = name, "{:#}", anyhow::Error::from_boxed(e));
                }
                return;
            }
            match result {
                Ok(()) => error!(
                    subsystem = name,
                    "critical task exited before shutdown was requested"
                ),
                Err(e) => {
                    error!(subsystem = name, "{:#}", anyhow::Error::from_boxed(e));
                }
            }

            executor.failed.store(true, Ordering::Relaxed);
            executor.trigger_shutdown();
        }))
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

    /// Wait for shutdown to be triggered, then await all tracked tasks.
    pub async fn wait_for_shutdown(&self) -> ShutdownReason {
        self.token.cancelled().await;
        self.tracker.close();
        self.tracker.wait().await;
        if self.failed.load(Ordering::Relaxed) {
            ShutdownReason::TaskFailed
        } else {
            ShutdownReason::Requested
        }
    }
}
