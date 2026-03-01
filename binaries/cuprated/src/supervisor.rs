use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::info;

use crate::error::CupratedError;

/// A handle for triggering shutdown.
#[derive(Clone)]
pub struct ShutdownHandle {
    token: CancellationToken,
    exit_code: Arc<AtomicU8>,
}

impl ShutdownHandle {
    /// Read the current exit code.
    pub fn exit_code(&self) -> u8 {
        self.exit_code.load(Ordering::Relaxed)
    }

    /// Get a clone of the cancellation token.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Returns a future that completes when shutdown is triggered.
    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }

    /// Trigger a graceful shutdown with the given exit code.
    pub fn trigger_shutdown(&self, exit_code: u8) {
        if !self.token.is_cancelled() {
            #[expect(clippy::let_underscore_must_use)]
            let _ =
                self.exit_code
                    .compare_exchange(0, exit_code, Ordering::Relaxed, Ordering::Relaxed);
            info!("Shutting down gracefully... Press Ctrl+C to exit immediately.");
        }
        self.token.cancel();
    }

    /// Report a service error and trigger a shutdown.
    pub fn report_service_error(&self, error: impl std::fmt::Display) {
        if self.token.is_cancelled() {
            return;
        }
        tracing::error!("{error}");
        self.trigger_shutdown(1);
    }
}

/// Used to wait for all tracked tasks after cancellation.
pub struct CupratedSupervisor {
    pub task_tracker: TaskTracker,
    pub shutdown_handle: ShutdownHandle,
}

/// A cloneable handle passed to tasks for spawning sub-tasks.
#[derive(Clone)]
pub struct CupratedTask {
    pub task_tracker: TaskTracker,
    pub shutdown_handle: ShutdownHandle,
}

impl CupratedTask {
    /// Spawn a task whose failure triggers a graceful shutdown.
    pub fn spawn_critical<F>(
        &self,
        fut: F,
        on_shutdown: impl FnOnce() + Send + 'static,
    ) -> JoinHandle<Result<(), CupratedError>>
    where
        F: Future<Output = Result<(), CupratedError>> + Send + 'static,
    {
        let handle = self.shutdown_handle.clone();
        self.task_tracker.spawn(async move {
            let result = fut.await;
            if let Err(ref e) = result {
                handle.report_service_error(e);
            }
            on_shutdown();
            result
        })
    }
}

/// Create a new [`CupratedSupervisor`] and [`CupratedTask`] pair.
///
/// Must be called exactly once at startup.
pub fn new() -> (CupratedSupervisor, CupratedTask) {
    let task_tracker = TaskTracker::new();
    let shutdown_handle = ShutdownHandle {
        token: CancellationToken::new(),
        exit_code: Arc::new(AtomicU8::new(0)),
    };

    (
        CupratedSupervisor {
            task_tracker: task_tracker.clone(),
            shutdown_handle: shutdown_handle.clone(),
        },
        CupratedTask {
            task_tracker,
            shutdown_handle,
        },
    )
}

/// Spawn a task that listens for OS signals and initiates shutdown.
pub fn spawn_signal_handler(handle: ShutdownHandle) {
    tokio::spawn(async move {
        tokio::select! {
            biased;
            () = handle.cancelled() => {}
            signal = shutdown_signal() => {
                eprintln!();
                handle.trigger_shutdown(signal);
            }
        }
        let second_signal = shutdown_signal().await;
        eprintln!();
        std::process::exit(i32::from(second_signal));
    });
}

/// Wait for an OS shutdown signal (SIGINT or SIGTERM).
async fn shutdown_signal() -> u8 {
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
        () = ctrl_c => 130,
        () = terminate => 143,
    }
}
