use cuprate_p2p::{self, peer::PeerError};
use monero_wire::levin;
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use futures::{StreamExt};
use std::{future::Future, pin::Pin};

struct Test;

impl tower::Service<cuprate_p2p::protocol::Request> for Test {
    type Response = Option<cuprate_p2p::protocol::Response>;
    type Error = PeerError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    fn call(&mut self, req: cuprate_p2p::protocol::Request) -> Self::Future {
        todo!()
    }
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn test() {
    let lis = tokio::net::TcpListener::bind("127.0.0.1:18088").await.unwrap();

    let (mut srm, adr) = lis.accept().await.unwrap();
    let (r, w) = srm.split();
    let mut mes_srm = levin::message_stream::MessageStream::<
        tokio_util::compat::Compat<tokio::net::tcp::ReadHalf>,
        monero_wire::Message,
    >::new(r.compat());
    let mut mes_snk = levin::message_sink::MessageSink::<
        tokio_util::compat::Compat<tokio::net::tcp::WriteHalf>,
        monero_wire::Message,
    >::new(w.compat_write());
    let svc = Test;
    let peer = cuprate_p2p::peer::connection::Connection::new(mes_snk, svc);
    let (tx, rx) = futures::channel::mpsc::channel(1);
    peer.run(mes_srm, rx).await;
}
