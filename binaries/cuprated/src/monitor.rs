//! Task spawning and shutdown coordination.

use std::{
    future::Future,
    panic::AssertUnwindSafe,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::FutureExt;
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{error, info};

use crate::constants::CRITICAL_SERVICE_ERROR;

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

    /// Spawn a tracked task that triggers shutdown if the future returns an
    /// error or panics.
    pub fn spawn_critical<F, E>(&self, name: &'static str, future: F) -> JoinHandle<()>
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Into<anyhow::Error> + Send + 'static,
    {
        let executor = self.clone();
        self.tracker.spawn(async move {
            match AssertUnwindSafe(future).catch_unwind().await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error!(subsystem = name, "{:#}", e.into());
                    executor.failed.store(true, Ordering::Relaxed);
                }
                Err(payload) => {
                    let msg = payload
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<no panic message>");
                    error!(subsystem = name, err = msg, "{CRITICAL_SERVICE_ERROR}");
                    executor.failed.store(true, Ordering::Relaxed);
                }
            }
            executor.trigger_shutdown();
        })
    }

    /// Get a clone of the cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Returns `true` if any critical task has failed (returned `Err` or panicked).
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
    pub(crate) fn close(&self) {
        self.tracker.close();
    }

    /// Wait for all tracked tasks to complete.
    pub(crate) async fn wait(&self) {
        self.tracker.wait().await;
    }
}
