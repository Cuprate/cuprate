use std::{
    pin::Pin,
    str::FromStr,
    sync::{Arc, Mutex},
};

use cuprate_common::{HardForks, Network};
use cuprate_peer::PeerError;
use cuprate_protocol::{
    temp_database::{BlockKnown, DataBaseRequest, DataBaseResponse, DatabaseError},
    Direction, InternalMessageRequest, InternalMessageResponse,
};
use cuprate_sync_states::SyncStates;
use futures::{channel::mpsc, Future, FutureExt};
use monero::Hash;
use monero_wire::messages::{admin::HandshakeResponse, CoreSyncData};
use tower::ServiceExt;

use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct TestBlockchain;

impl tower::Service<DataBaseRequest> for TestBlockchain {
    type Error = DatabaseError;
    type Response = DataBaseResponse;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: DataBaseRequest) -> Self::Future {
        let res = match req {
            DataBaseRequest::BlockHeight(h) => DataBaseResponse::BlockHeight(Some(221)),
            DataBaseRequest::BlockKnown(_) => DataBaseResponse::BlockKnown(BlockKnown::OnMainChain),
            DataBaseRequest::Chain => todo!(),
            DataBaseRequest::CoreSyncData => {
                DataBaseResponse::CoreSyncData(CoreSyncData::new(0, 0, 0, Hash::null(), 0))
            }
            DataBaseRequest::CumulativeDifficulty => DataBaseResponse::CumulativeDifficulty(0),
            DataBaseRequest::CurrentHeight => DataBaseResponse::CurrentHeight(0),
        };

        async { Ok(res) }.boxed()
    }
}

#[derive(Debug, Clone)]
struct TestPeerRequest;

impl tower::Service<InternalMessageRequest> for TestPeerRequest {
    type Error = PeerError;
    type Response = InternalMessageResponse;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }
    fn call(&mut self, req: InternalMessageRequest) -> Self::Future {
        todo!()
    }
}

#[tokio::test]
async fn test_p2p_conn() {
    let conf = cuprate_peer::handshaker::NetworkConfig::default();
    let (addr_tx, addr_rx) = mpsc::channel(21);
    let (sync_tx, sync_rx) = mpsc::channel(21);
    let peer_sync_states = Arc::new(Mutex::default());

    let peer_sync_states = SyncStates::new(
        sync_rx,
        HardForks::new(Network::MainNet),
        peer_sync_states,
        TestBlockchain,
    );

    let mut handshaker = cuprate_peer::handshaker::Handshaker::new(
        conf,
        addr_tx,
        TestBlockchain,
        sync_tx,
        TestPeerRequest.boxed_clone(),
    );

    let soc = tokio::net::TcpSocket::new_v4().unwrap();
    let addr = std::net::SocketAddr::from_str("127.0.0.1:18080").unwrap();

    let mut con = soc.connect(addr).await.unwrap();

    let (r_h, w_h) = con.split();

    let (client, conn) = handshaker
        .complete_handshake(
            r_h.compat(),
            w_h.compat_write(),
            Direction::Outbound,
            monero_wire::NetworkAddress::default(),
        )
        .await
        .unwrap();

    //conn.run().await;
}
