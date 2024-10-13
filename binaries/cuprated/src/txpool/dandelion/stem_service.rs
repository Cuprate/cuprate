use std::{
    pin::Pin,
    task::{Context, Poll},
};

use cuprate_dandelion_tower::{traits::StemRequest, OutboundPeer};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{
    client::{Client, InternalPeerID},
    ClearNet, NetworkZone, PeerRequest, ProtocolRequest,
};
use cuprate_wire::{protocol::NewTransactions, NetworkAddress};

use bytes::Bytes;
use futures::Stream;
use tower::Service;

use super::DandelionTx;

/// The dandelion outbound peer stream.
pub struct OutboundPeerStream {
    pub clear_net: NetworkInterface<ClearNet>,
}

impl Stream for OutboundPeerStream {
    type Item = Result<OutboundPeer<NetworkAddress, StemPeerService<ClearNet>>, tower::BoxError>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // TODO: make the outbound peer choice random.
        Poll::Ready(Some(Ok(self
            .clear_net
            .client_pool()
            .outbound_client()
            .map_or(OutboundPeer::Exhausted, |client| {
                let addr = match client.info.id {
                    InternalPeerID::KnownAddr(addr) => addr,
                    InternalPeerID::Unknown(_) => panic!("Outbound peer had an unknown address"),
                };

                OutboundPeer::Peer(addr.into(), StemPeerService(client))
            }))))
    }
}

/// The stem service, used to send stem txs.
pub struct StemPeerService<N: NetworkZone>(Client<N>);

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
