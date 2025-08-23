use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures::{Stream, StreamExt, TryStream};
use tower::Service;

use cuprate_dandelion_tower::{DandelionRouterError, OutboundPeer};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{client::InternalPeerID, NetworkZone};

use crate::{
    p2p::CrossNetworkInternalPeerId,
    txpool::dandelion::stem_service::{OutboundPeerStream, StemPeerService},
};

/// The service to prepare peers on anonymous network zones for sending transactions.
pub struct AnonTxService<Z: NetworkZone> {
    outbound_peer_discover: Pin<Box<OutboundPeerStream<Z>>>,
    pub peer: Option<StemPeerService<Z>>,
}

impl<Z: NetworkZone> AnonTxService<Z>
where
    InternalPeerID<Z::Addr>: Into<CrossNetworkInternalPeerId>,
{
    pub fn new(network_interface: NetworkInterface<Z>) -> Self {
        Self {
            outbound_peer_discover: Box::pin(OutboundPeerStream::new(network_interface)),
            peer: None,
        }
    }

    pub fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), DandelionRouterError>> {
        loop {
            if let Some(peer) = &mut self.peer {
                if ready!(peer.poll_ready(cx)).is_err() {
                    self.peer = None;

                    continue;
                }

                return Poll::Ready(Ok(()));
            }

            let ret = ready!(self
                .outbound_peer_discover
                .as_mut()
                .try_poll_next(cx)
                .map_err(DandelionRouterError::OutboundPeerStreamError))
            .ok_or(DandelionRouterError::OutboundPeerDiscoverExited)??;

            match ret {
                OutboundPeer::Peer(_, mut svc) => {
                    let poll = svc.poll_ready(cx);
                    self.peer = Some(svc);
                    if ready!(poll).is_err() {
                        self.peer = None;
                    }
                }
                OutboundPeer::Exhausted => return Poll::Ready(Ok(())),
            }
        }

        Poll::Ready(Ok(()))
    }
}
