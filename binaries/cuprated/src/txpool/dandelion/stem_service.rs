use super::DandelionTx;
use bytes::Bytes;
use cuprate_dandelion_tower::traits::StemRequest;
use cuprate_dandelion_tower::OutboundPeer;
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::client::Client;
use cuprate_p2p_core::{ClearNet, NetworkZone, PeerRequest, ProtocolRequest};
use cuprate_wire::protocol::NewTransactions;
use cuprate_wire::NetworkAddress;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

pub struct OutboundPeerStream {
    pub clear_net: NetworkInterface<ClearNet>,
}

impl Stream for OutboundPeerStream {
    type Item = Result<OutboundPeer<NetworkAddress, StemPeerService<ClearNet>>, tower::BoxError>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(Ok(self
            .clear_net
            .client_pool()
            .outbound_client()
            .map_or(OutboundPeer::Exhausted, |client| {
                OutboundPeer::Peer(client.info.id.into(), StemPeerService(client))
            }))))
    }
}

pub struct StemPeerService<N>(Client<N>);

impl<N: NetworkZone> Service<StemRequest<DandelionTx>> for StemPeerService<N> {
    type Response = ();
    type Error = tower::BoxError;
    type Future = Client::Future;

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
