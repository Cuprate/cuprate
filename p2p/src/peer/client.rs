use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::task::{Context, Poll};
use std::{future::Future, sync::Arc};

use futures::{
    channel::{mpsc, oneshot},
    FutureExt,
};
use tokio::task::JoinHandle;
use tower::BoxError;

use cuprate_common::PruningSeed;
use monero_wire::{messages::common::PeerSupportFlags, NetworkAddress};

use super::{
    connection::ClientRequest,
    error::{ErrorSlot, PeerError, SharedPeerError},
    PeerError,
};
use crate::connection_handle::PeerHandle;
use crate::protocol::{InternalMessageRequest, InternalMessageResponse};

pub struct ConnectionInfo {
    pub support_flags: PeerSupportFlags,
    pub pruning_seed: PruningSeed,
    pub handle: PeerHandle,
    pub rpc_port: u16,
    pub rpc_credits_per_hash: u32,
}

pub struct Client {
    pub connection_info: Arc<ConnectionInfo>,
    /// Used to shut down the corresponding heartbeat.
    /// This is always Some except when we take it on drop.
    heartbeat_shutdown_tx: Option<oneshot::Sender<()>>,
    server_tx: mpsc::Sender<ClientRequest>,
    connection_task: JoinHandle<()>,
    heartbeat_task: JoinHandle<()>,

    error_slot: ErrorSlot,
}

impl Client {
    pub fn new(
        connection_info: Arc<ConnectionInfo>,
        heartbeat_shutdown_tx: oneshot::Sender<()>,
        server_tx: mpsc::Sender<ClientRequest>,
        connection_task: JoinHandle<()>,
        heartbeat_task: JoinHandle<()>,
        error_slot: ErrorSlot,
    ) -> Self {
        Client {
            connection_info,
            heartbeat_shutdown_tx: Some(heartbeat_shutdown_tx),
            server_tx,
            connection_task,
            heartbeat_task,
            error_slot,
        }
    }

    /// Check if this connection's heartbeat task has exited.
    #[allow(clippy::unwrap_in_result)]
    fn check_heartbeat(&mut self, cx: &mut Context<'_>) -> Result<(), SharedPeerError> {
        let is_canceled = self
            .heartbeat_shutdown_tx
            .as_mut()
            .expect("only taken on drop")
            .poll_canceled(cx)
            .is_ready();

        if is_canceled {
            return self.set_task_exited_error(
                "heartbeat",
                PeerError::HeartbeatTaskExited("Task was cancelled".to_string()),
            );
        }

        match self.heartbeat_task.poll_unpin(cx) {
            Poll::Pending => {
                // Heartbeat task is still running.
                Ok(())
            }
            Poll::Ready(Ok(Ok(_))) => {
                // Heartbeat task stopped unexpectedly, without panic or error.
                self.set_task_exited_error(
                    "heartbeat",
                    PeerError::HeartbeatTaskExited(
                        "Heartbeat task stopped unexpectedly".to_string(),
                    ),
                )
            }
            Poll::Ready(Ok(Err(error))) => {
                // Heartbeat task stopped unexpectedly, with error.
                self.set_task_exited_error(
                    "heartbeat",
                    PeerError::HeartbeatTaskExited(error.to_string()),
                )
            }
            Poll::Ready(Err(error)) => {
                // Heartbeat task was cancelled.
                if error.is_cancelled() {
                    self.set_task_exited_error(
                        "heartbeat",
                        PeerError::HeartbeatTaskExited("Task was cancelled".to_string()),
                    )
                }
                // Heartbeat task stopped with panic.
                else if error.is_panic() {
                    panic!("heartbeat task has panicked: {error}");
                }
                // Heartbeat task stopped with error.
                else {
                    self.set_task_exited_error(
                        "heartbeat",
                        PeerError::HeartbeatTaskExited(error.to_string()),
                    )
                }
            }
        }
    }

    /// Check if the connection's task has exited.
    fn check_connection(&mut self, context: &mut Context<'_>) -> Result<(), PeerError> {
        match self.connection_task.poll_unpin(context) {
            Poll::Pending => {
                // Connection task is still running.
                Ok(())
            }
            Poll::Ready(Ok(())) => {
                // Connection task stopped unexpectedly, without panicking.
                return Err(PeerError::ConnectionTaskClosed);
            }
            Poll::Ready(Err(error)) => {
                // Connection task stopped unexpectedly with a panic. shut the node down.
                tracing::error!("Peer Connection task panicked: {error}, shutting the node down!");
                set_shutting_down();
                return Err(PeerError::ConnectionTaskClosed);
            }
        }
    }
}

impl tower::Service<InternalMessageRequest> for Client {
    type Response = InternalMessageResponse;
    type Error = SharedPeerError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.server_tx
            .poll_ready(cx)
            .map_err(|e| PeerError::ClientChannelClosed.into())
    }
    fn call(&mut self, req: InternalMessageRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        match self.server_tx.try_send(ClientRequest { req, tx }) {
            Ok(()) => rx
                .map(|recv_result| {
                    recv_result
                        .expect("ClientRequest oneshot sender must not be dropped before send")
                        .map_err(|e| e.into())
                })
                .boxed(),
            Err(_) => {
                // TODO: better error handling
                futures::future::ready(Err(PeerError::ClientChannelClosed.into())).boxed()
            }
        }
    }
}
