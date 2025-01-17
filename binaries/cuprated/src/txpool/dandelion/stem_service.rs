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
    ClearNet, NetworkZone, PeerRequest, ProtocolRequest,
};
use cuprate_wire::protocol::NewTransactions;

use crate::{p2p::CrossNetworkInternalPeerId, txpool::dandelion::DandelionTx};

/// The dandelion outbound peer stream.
pub struct OutboundPeerStream {
    clear_net: NetworkInterface<ClearNet>,
    state: OutboundPeerStreamState,
}

impl OutboundPeerStream {
    pub const fn new(clear_net: NetworkInterface<ClearNet>) -> Self {
        Self {
            clear_net,
            state: OutboundPeerStreamState::Standby,
        }
    }
}

impl Stream for OutboundPeerStream {
    type Item = Result<
        OutboundPeer<CrossNetworkInternalPeerId, StemPeerService<ClearNet>>,
        tower::BoxError,
    >;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &mut self.state {
                OutboundPeerStreamState::Standby => {
                    let peer_set = self.clear_net.peer_set();
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
                            Some(peer) => OutboundPeer::Peer(
                                CrossNetworkInternalPeerId::ClearNet(peer.info.id),
                                StemPeerService(peer),
                            ),
                            None => OutboundPeer::Exhausted,
                        }
                    })));
                }
            }
        }
    }
}

/// The state of the [`OutboundPeerStream`].
enum OutboundPeerStreamState {
    /// Standby state.
    Standby,
    /// Awaiting a response from the peer-set.
    AwaitingPeer(BoxFuture<'static, Result<PeerSetResponse<ClearNet>, tower::BoxError>>),
}

/// The stem service, used to send stem txs.
pub struct StemPeerService<N: NetworkZone>(ClientDropGuard<N>);

impl<N: NetworkZone> Service<StemRequest<DandelionTx>> for StemPeerService<N> {
    type Response = <Client<N> as Service<PeerRequest>>::Response;
    type Error = tower::BoxError;
    type Future = <Client<N> as Service<PeerRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, req: StemRequest<DandelionTx>) -> Self::Future {
        self.0
            .call(PeerRequest::Protocol(ProtocolRequest::NewTransactions(
                NewTransactions {
                    txs: vec![req.0 .0],
                    dandelionpp_fluff: false,
                    padding: Bytes::new(),
                },
            )))
    }
}
