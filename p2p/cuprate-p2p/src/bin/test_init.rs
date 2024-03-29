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

pub struct DummyBlockchain;

impl Blockchain for DummyBlockchain {
    fn top_hash(&mut self) -> impl Future<Output = [u8; 32]> + Send {
        async {
            hex::decode("418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3")
                .unwrap()
                .try_into()
                .unwrap()
        }
    }

    fn chain_history(
        &mut self,
        from: Option<[u8; 32]>,
    ) -> impl Future<Output = Vec<[u8; 32]>> + Send {
        async {
            vec![
                hex::decode("418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3")
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ]
        }
    }

    fn cumulative_difficulty(&mut self) -> impl Future<Output = u128> + Send {
        async { 1 }
    }

    fn have_block(&mut self, block_id: [u8; 32]) -> impl Future<Output = Where> + Send {
        async move {
            if block_id.as_ref()
                == hex::decode("418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3")
                    .unwrap()
            {
                return Where::MainChain(0);
            }

            Where::NotFound
        }
    }

    fn current_height(&mut self) -> impl Future<Output = u64> + Send {
        async { 1 }
    }
}

use cuprate_helper::network::Network;
use cuprate_p2p::block_downloader::{download_blocks, BlockDownloader2, Blockchain, Where};
use cuprate_p2p::broadcast::BroadcastConfig;
use cuprate_p2p::config::P2PConfig;
use cuprate_p2p::{broadcast, init_network};
use futures::{FutureExt, StreamExt};
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
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::sleep;
use tower::Service;
use tracing::metadata::LevelFilter;
use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::from_level(Level::INFO))
        //  .pretty()
        .with_line_number(false)
        .with_file(false)
        .with_target(false)
        .finish()
        .init();

    let address_book_cfg = monero_address_book::AddressBookConfig {
        max_white_list_length: 1000,
        max_gray_list_length: 50000,
        peer_store_file: PathBuf::new().join("p2p_store"),
        peer_save_period: Duration::from_secs(60),
    };

    let cfg = P2PConfig {
        p2p_port: 0,
        rpc_port: 0,
        network: Default::default(),
        outbound_connections: 60,
        max_outbound_connections: 80,
        anchor_connections: 10,
        gray_peers_percent: 0.7,
        max_inbound_connections: 125,
        address_book_config: address_book_cfg.clone(),
        broadcast_config: Default::default(),
    };

    let mut network =
        init_network::<ClearNet, _, _>(&cfg, DummyCoreSyncSvc, DummyPeerRequestHandlerSvc)
            .await
            .unwrap();

    network.top_sync_data_watch.changed().await.unwrap();

    sleep(Duration::from_secs(15)).await;

    let mut buffer =
        download_blocks(network.peer_sync_svc, network.peer_set, DummyBlockchain).await;

    while let Some(blocks) = buffer.next().await {
        tracing::info!(
            "{}, {}",
            hex::encode(blocks[0].0.hash()),
            blocks[0].0.number().unwrap()
        );
    }

    tracing::info!("{:?}", network.top_sync_data_watch.borrow())
}
