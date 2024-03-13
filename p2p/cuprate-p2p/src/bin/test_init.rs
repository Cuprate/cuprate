#[derive(Clone)]
pub struct DummyCoreSyncSvc;

impl Service<CoreSyncDataRequest> for DummyCoreSyncSvc {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: CoreSyncDataRequest) -> Self::Future {
        async move {
            Ok(CoreSyncDataResponse(monero_wire::CoreSyncData {
                cumulative_difficulty: 1,
                cumulative_difficulty_top64: 0,
                current_height: 1,
                pruning_seed: 0,
                top_id: hex::decode(
                    "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                top_version: 1,
            }))
        }
        .boxed()
    }
}

#[derive(Clone)]
pub struct DummyPeerSyncSvc;

impl<N: NetworkZone> Service<PeerSyncRequest<N>> for DummyPeerSyncSvc {
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    type Response = PeerSyncResponse<N>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: PeerSyncRequest<N>) -> Self::Future {
        async { Ok(PeerSyncResponse::Ok) }.boxed()
    }
}

#[derive(Clone)]
pub struct DummyPeerRequestHandlerSvc;

impl Service<PeerRequest> for DummyPeerRequestHandlerSvc {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: PeerRequest) -> Self::Future {
        async move { Ok(PeerResponse::NA) }.boxed()
    }
}

use cuprate_helper::network::Network;
use cuprate_p2p::config::P2PConfig;
use futures::FutureExt;
use monero_p2p::client::{Client, Connector, HandShaker};
use monero_p2p::network_zones::ClearNet;
use monero_p2p::services::{
    CoreSyncDataRequest, CoreSyncDataResponse, PeerSyncRequest, PeerSyncResponse,
};
use monero_p2p::{NetworkZone, PeerRequest, PeerResponse};
use monero_wire::common::PeerSupportFlags;
use monero_wire::BasicNodeData;
use rand::distributions::Bernoulli;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tower::Service;
use tracing::metadata::LevelFilter;
use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(Semaphore::new(100));

    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::from_level(Level::DEBUG))
        .finish()
        .init();

    let our_basic_node_data = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let address_book_cfg = monero_address_book::AddressBookConfig {
        max_white_list_length: 1000,
        max_gray_list_length: 50000,
        peer_store_file: PathBuf::new().join("p2p_store"),
        peer_save_period: Duration::from_secs(600000),
    };

    let cfg = P2PConfig {
        outbound_connections: 100,
        max_outbound_connections: 150,
        anchor_connections: 10,
        gray_peers_percent: 0.7,
        max_inbound_connections: 125,
        address_book_config: address_book_cfg.clone(),
    };

    let addr_book_svc = monero_address_book::init_address_book::<ClearNet>(address_book_cfg)
        .await
        .unwrap();

    let handshaker = HandShaker::<ClearNet, _, _, _, _>::new(
        addr_book_svc.clone(),
        DummyCoreSyncSvc,
        DummyPeerSyncSvc,
        DummyPeerRequestHandlerSvc,
        None,
        our_basic_node_data,
    );

    let connector = Connector::new(handshaker);

    let (tx, _rx) = mpsc::channel::<Client<ClearNet>>(10);
    let (_tx2, rx2) = mpsc::channel(10);

    let conn = cuprate_p2p::connection_maintainer::OutboundConnectionKeeper {
        new_connection_tx: tx,
        make_connection_rx: rx2,
        address_book_svc: addr_book_svc,
        connector_svc: connector,
        outbound_semaphore: semaphore,
        extra_peers: 0,
        config: cfg,
        peer_type_gen: Bernoulli::new(0.7).unwrap(),
    };

    conn.run().await
}
