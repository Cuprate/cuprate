use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::{future::Future, sync::Arc};

use futures::{
    channel::{mpsc, oneshot},
    FutureExt,
};
use tower::BoxError;

use crate::peer::handshaker::ConnectionAddr;
use cuprate_common::PruningSeed;
use monero_wire::messages::PeerID;
use monero_wire::{messages::common::PeerSupportFlags, NetworkAddress};

use super::{connection::ClientRequest, PeerError};
use crate::protocol::{InternalMessageRequest, InternalMessageResponse};

pub struct ConnectionInfo {
    pub addr: ConnectionAddr,
    pub support_flags: PeerSupportFlags,
    pub pruning_seed: PruningSeed,
    pub peer_height: std::sync::Arc<AtomicU64>,
    /// Peer ID
    pub peer_id: PeerID,
    pub rpc_port: u16,
    pub rpc_credits_per_hash: u32,
}

pub struct Client {
    pub connection_info: Arc<ConnectionInfo>,
    server_tx: mpsc::Sender<ClientRequest>,
}

impl Client {
    pub fn new(
        connection_info: Arc<ConnectionInfo>,
        server_tx: mpsc::Sender<ClientRequest>,
    ) -> Self {
        Client {
            connection_info,
            server_tx,
        }
    }
}

impl tower::Service<InternalMessageRequest> for Client {
    type Response = InternalMessageResponse;
    type Error = BoxError;
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
            Err(_e) => {
                // TODO: better error handling
                futures::future::ready(Err(PeerError::ClientChannelClosed.into())).boxed()
            }
        }
    }
}
