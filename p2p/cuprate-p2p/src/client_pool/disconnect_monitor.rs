//! # Disconnect Monitor
//!
//! This module contains the [`disconnect_monitor`] task, which monitors connected peers for disconnection
//! and then removes them from the [`ClientPool`] if they do.
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::WaitForCancellationFutureOwned;
use tracing::instrument;

use monero_p2p::{client::InternalPeerID, handles::ConnectionHandle, NetworkZone};

use super::ClientPool;

/// The disconnect monitor task.
#[instrument(level = "info", skip_all)]
pub async fn disconnect_monitor<N: NetworkZone>(
    mut new_connection_rx: mpsc::UnboundedReceiver<(ConnectionHandle, InternalPeerID<N::Addr>)>,
    client_pool: Arc<ClientPool<N>>,
) {
    // We need to hold a weak reference otherwise the client pool and this would hold a reference to
    // each other causing the pool to be leaked.
    let weak_client_pool = Arc::downgrade(&client_pool);
    drop(client_pool);

    tracing::info!("Starting peer disconnect monitor.");

    let mut futs: FuturesUnordered<PeerDisconnectFut<N>> = FuturesUnordered::new();

    loop {
        tokio::select! {
            Some((con_handle, peer_id)) = new_connection_rx.recv() => {
                tracing::debug!("Monitoring {peer_id} for disconnect");
                futs.push(PeerDisconnectFut {
                    closed_fut: con_handle.closed(),
                    peer_id: Some(peer_id),
                });
            }
            Some(peer_id) = futs.next() => {
                tracing::debug!("{peer_id} has disconnected, removing from client pool.");
                let Some(pool) = weak_client_pool.upgrade() else {
                    tracing::info!("Peer disconnect monitor shutting down.");
                    return;
                };

                pool.remove_client(&peer_id);
                drop(pool);
            }
            else => {
                tracing::info!("Peer disconnect monitor shutting down.");
                return;
            }
        }
    }
}

/// A [`Future`] that resolves when a peer disconnects.
#[pin_project::pin_project]
pub(crate) struct PeerDisconnectFut<N: NetworkZone> {
    /// The inner [`Future`] that resolves when a peer disconnects.
    #[pin]
    pub(crate) closed_fut: WaitForCancellationFutureOwned,
    /// The peers ID.
    pub(crate) peer_id: Option<InternalPeerID<N::Addr>>,
}

impl<N: NetworkZone> Future for PeerDisconnectFut<N> {
    type Output = InternalPeerID<N::Addr>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        this.closed_fut
            .poll(cx)
            .map(|_| this.peer_id.take().unwrap())
    }
}
