use std::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt, Stream};
use tower::Service;

use cuprate_dandelion_tower::{traits::StemRequest, OutboundPeer};
use cuprate_p2p::{ClientDropGuard, NetworkInterface, PeerSetRequest, PeerSetResponse};
use cuprate_p2p_core::{
    client::{Client, InternalPeerID},
    BroadcastMessage, ClearNet, NetworkZone, PeerRequest, ProtocolRequest, Tor,
};
use cuprate_wire::protocol::NewTransactions;

use crate::{p2p::CrossNetworkInternalPeerId, txpool::dandelion::DandelionTx};

/// The dandelion outbound peer stream.
pub struct OutboundPeerStream<Z: NetworkZone> {
    network_interface: NetworkInterface<Z>,
    state: OutboundPeerStreamState<Z>,
}

impl<Z: NetworkZone> OutboundPeerStream<Z> {
    pub const fn new(network_interface: NetworkInterface<Z>) -> Self {
        Self {
            network_interface,
            state: OutboundPeerStreamState::Standby,
        }
    }
}

impl<Z: NetworkZone> Stream for OutboundPeerStream<Z>
where
    InternalPeerID<Z::Addr>: Into<CrossNetworkInternalPeerId>,
{
    type Item =
        Result<OutboundPeer<CrossNetworkInternalPeerId, StemPeerService<Z>>, tower::BoxError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &mut self.state {
                OutboundPeerStreamState::Standby => {
                    let peer_set = self.network_interface.peer_set();
                    let res = ready!(peer_set.poll_ready(cx));

                    self.state = OutboundPeerStreamState::AwaitingPeer(
                        peer_set.call(PeerSetRequest::StemPeer).boxed(),
                    );
                }
                OutboundPeerStreamState::AwaitingPeer(fut) => {
                    let res = ready!(fut.poll_unpin(cx));

                    self.state = OutboundPeerStreamState::Standby;

                    return Poll::Ready(Some(res.map(|res| {
                        let PeerSetResponse::StemPeer(stem_peer) = res else {
                            unreachable!()
                        };

                        match stem_peer {
                            Some(peer) => {
                                OutboundPeer::Peer(peer.info.id.into(), StemPeerService(peer))
                            }
                            None => OutboundPeer::Exhausted,
                        }
                    })));
                }
            }
        }
    }
}

/// The state of the [`OutboundPeerStream`].
enum OutboundPeerStreamState<Z: NetworkZone> {
    /// Standby state.
    Standby,
    /// Awaiting a response from the peer-set.
    AwaitingPeer(BoxFuture<'static, Result<PeerSetResponse<Z>, tower::BoxError>>),
}

/// The stem service, used to send stem txs.
pub struct StemPeerService<N: NetworkZone>(ClientDropGuard<N>);

impl<N: NetworkZone> Service<StemRequest<DandelionTx>> for StemPeerService<N> {
    type Response = <Client<N> as Service<PeerRequest>>::Response;
    type Error = tower::BoxError;
    type Future = <Client<N> as Service<PeerRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.broadcast_client().poll_ready(cx)
    }

    fn call(&mut self, req: StemRequest<DandelionTx>) -> Self::Future {
        self.0
            .broadcast_client()
            .call(BroadcastMessage::NewTransactions(NewTransactions {
                txs: vec![req.0 .0],
                dandelionpp_fluff: false,
                padding: Bytes::new(),
            }))
    }
}
