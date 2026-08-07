//! Task spawning and shutdown coordination.

use std::{future::Future, panic::AssertUnwindSafe};

use futures::FutureExt;
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{debug, error, info};

use crate::constants::CRITICAL_SERVICE_ERROR;

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

    /// Spawn a tracked task that triggers shutdown if the future returns
    /// early or panics.
    pub fn spawn_critical<F, E>(&self, name: &'static str, future: F) -> JoinHandle<()>
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Into<anyhow::Error> + Send + 'static,
    {
        let executor = self.clone();
        self.tracker
            .spawn(AssertUnwindSafe(future).catch_unwind().map(move |result| {
                match result {
                    Ok(res) => {
                        if executor.token.is_cancelled() {
                            // Node is shutting down, so an early exit or error is expected
                            if let Err(e) = res {
                                debug!(subsystem = name, "{:#}", e.into());
                            }
                            return;
                        }
                        match res {
                            Ok(()) => error!(
                                subsystem = name,
                                "critical task exited before shutdown was requested"
                            ),
                            Err(e) => error!(subsystem = name, "{:#}", e.into()),
                        }
                    }
                    Err(payload) => error!(
                        subsystem = name,
                        err = panic_message(&payload),
                        "{CRITICAL_SERVICE_ERROR}",
                    ),
                }
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
    pub async fn wait_for_shutdown(&self) {
        self.token.cancelled().await;
        self.tracker.close();
        self.tracker.wait().await;
    }
}

/// Extracts a printable message from a `catch_unwind` panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .unwrap_or("<no panic message>")
}
