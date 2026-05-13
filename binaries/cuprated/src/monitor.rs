//! Task spawning and shutdown coordination.

use std::future::Future;

use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::info;

/// A handle for task spawning and shutdown coordination.
#[derive(Clone, Default)]
pub struct TaskExecutor {
    token: CancellationToken,
    tracker: TaskTracker,
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
    pub(crate) fn close(&self) {
        self.tracker.close();
    }

    /// Wait for all tracked tasks to complete.
    pub(crate) async fn wait(&self) {
        self.tracker.wait().await;
    }
}
