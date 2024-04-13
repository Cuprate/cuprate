use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::WaitForCancellationFutureOwned;
use tracing::instrument;

use monero_p2p::{client::InternalPeerID, handles::ConnectionHandle, NetworkZone};

use super::ClientPool;

#[instrument(level="info", skip_all, fields(network=N::NAME))]
pub async fn disconnect_monitor<N: NetworkZone>(
    mut new_connection_rx: mpsc::UnboundedReceiver<(ConnectionHandle, InternalPeerID<N::Addr>)>,
    client_pool: Arc<ClientPool<N>>,
) {
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
                tracing::debug!("{peer_id} has disconnecting, removing from peer_set.");
                client_pool.remove_client(&peer_id);
            }
            else => {
                tracing::info!("Peer disconnect monitor shutting down.");
                return;
            }
        }
    }
}

#[pin_project::pin_project]
struct PeerDisconnectFut<N: NetworkZone> {
    #[pin]
    closed_fut: WaitForCancellationFutureOwned,
    peer_id: Option<InternalPeerID<N::Addr>>,
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
