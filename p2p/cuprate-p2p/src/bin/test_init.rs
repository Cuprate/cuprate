use cuprate_blockchain::config::ReaderThreads;
use cuprate_consensus::context::NewBlockData;
use cuprate_consensus::transactions::TransactionVerificationData;
use cuprate_consensus::{
    generate_genesis_block, BlockChainContextRequest, VerifyBlockRequest, VerifyBlockResponse,
    VerifyTxRequest,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_helper::network::Network;
use cuprate_p2p::block_downloader::{
    download_blocks, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse,
};
use cuprate_p2p::{initialize_network, P2PConfig};
use cuprate_types::blockchain::BCWriteRequest;
use cuprate_types::VerifiedBlockInformation;
use futures::{FutureExt, StreamExt};
use monero_address_book::AddressBookConfig;
use monero_p2p::network_zones::ClearNet;
use monero_p2p::services::{CoreSyncDataRequest, CoreSyncDataResponse};
use monero_p2p::{PeerRequest, PeerResponse};
use monero_wire::admin::TimedSyncResponse;
use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::sleep;
use tower::{Service, ServiceExt};
use tracing::Level;
use tracing_subscriber::fmt::time::Uptime;

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
pub struct DummyPeerRequestHandlerSvc;

impl Service<PeerRequest> for DummyPeerRequestHandlerSvc {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PeerRequest) -> Self::Future {
        async move {
            Ok(match req {
                PeerRequest::TimedSync(_) => PeerResponse::TimedSync(TimedSyncResponse {
                    payload_data: monero_wire::CoreSyncData {
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
                    },
                    local_peerlist_new: vec![],
                }),
                _ => PeerResponse::NA,
            })
        }
        .boxed()
    }
}

pub struct OurChainSvc;

impl Service<ChainSvcRequest> for OurChainSvc {
    type Response = ChainSvcResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ChainSvcRequest) -> Self::Future {
        async move {
            Ok(match req {
                ChainSvcRequest::CompactHistory => ChainSvcResponse::CompactHistory {
                    block_ids: vec![hex::decode(
                        "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
                    )
                    .unwrap()
                    .try_into()
                    .unwrap()],
                    cumulative_difficulty: 1,
                },
                ChainSvcRequest::FindFirstUnknown(_) => ChainSvcResponse::FindFirstUnknown(1),
                ChainSvcRequest::CumulativeDifficulty => ChainSvcResponse::CumulativeDifficulty(1),
            })
        }
        .boxed()
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 12)]
async fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(12)
        .build_global()
        .unwrap();

    let (database_read, mut database_write) = cuprate_blockchain::service::init(
        cuprate_blockchain::config::ConfigBuilder::new()
            .fast()
            .reader_threads(ReaderThreads::Number(8))
            .build(),
    )
    .unwrap();

    let genesis_block = generate_genesis_block(&Network::Mainnet);

    let verified_block = VerifiedBlockInformation {
        block_blob: genesis_block.serialize(),
        txs: vec![],
        block_hash: genesis_block.hash(),
        pow_hash: hex::decode("8a7b1a780e99eec31a9425b7d89c283421b2042a337d5700dfd4a7d6eb7bd774")
            .unwrap()
            .try_into()
            .unwrap(),
        height: 0,
        generated_coins: genesis_block.miner_tx.prefix.outputs[0].amount.unwrap(),
        weight: genesis_block.miner_tx.weight(),
        long_term_weight: genesis_block.miner_tx.weight(),
        cumulative_difficulty: 1,
        block: genesis_block,
    };

    database_write
        .ready()
        .await
        .unwrap()
        .call(BCWriteRequest::WriteBlock(verified_block))
        .await
        .unwrap();

    let mut context_svc = cuprate_consensus::context::initialize_blockchain_context(
        cuprate_consensus::ContextConfig::main_net(),
        database_read.clone().map_err(Into::into),
    )
    .await
    .unwrap();

    let (mut block_verifier, _) = cuprate_consensus::initialize_verifier(
        database_read.clone().map_err(Into::into),
        context_svc.clone(),
    )
    .await
    .unwrap();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_timer(Uptime::default())
        .init();

    let config = P2PConfig::<ClearNet> {
        network: Default::default(),
        outbound_connections: 32,
        extra_outbound_connections: 32,
        max_inbound_connections: 128,
        gray_peers_percent: 0.7,
        server_config: None,
        p2p_port: 0,
        rpc_port: 0,
        address_book_config: AddressBookConfig {
            max_white_list_length: 1000,
            max_gray_list_length: 5000,
            peer_store_file: PathBuf::from_str("p2p_store").unwrap(),
            peer_save_period: Duration::from_secs(300),
        },
    };

    let net = initialize_network(DummyPeerRequestHandlerSvc, DummyCoreSyncSvc, config)
        .await
        .unwrap();

    sleep(Duration::from_secs(15)).await;

    let mut buffer = download_blocks(
        net.pool.clone(),
        net.sync_states_svc.clone(),
        OurChainSvc,
        BlockDownloaderConfig {
            buffer_size: 50_000_000,
            in_progress_queue_size: 30_000_000,
            check_client_pool_interval: Duration::from_secs(30),
            target_batch_size: 8_000_000,
            initial_batch_size: 10,
        },
    );

    while let Some(entry) = buffer.next().await {
        tracing::info!(
            "new batch height: {}, amount{}",
            entry.blocks[0].0.number().unwrap(),
            entry.blocks.len()
        );

        tracing::info!("Prepping {} blocks for verification", entry.blocks.len());

        let VerifyBlockResponse::MainChainBatchPrepped(blocks) = block_verifier
            .ready()
            .await
            .unwrap()
            .call(VerifyBlockRequest::MainChainBatchPrepareBlocks {
                blocks: entry.blocks,
            })
            .await
            .unwrap()
        else {
            panic!()
        };

        for (block, txs) in blocks {
            let VerifyBlockResponse::MainChain(block_info) = block_verifier
                .ready()
                .await
                .unwrap()
                .call(VerifyBlockRequest::MainChainPrepped { block, txs })
                .await
                .unwrap()
            else {
                panic!()
            };

            let height = block_info.height;

            let info = NewBlockData {
                block_hash: block_info.block_hash,
                height: block_info.height,
                timestamp: block_info.block.header.timestamp,
                weight: block_info.weight,
                long_term_weight: block_info.long_term_weight,
                generated_coins: block_info.generated_coins,
                vote: cuprate_consensus::HardFork::from_vote(block_info.block.header.minor_version),
                cumulative_difficulty: block_info.cumulative_difficulty,
            };

            database_write
                .ready()
                .await
                .unwrap()
                .call(BCWriteRequest::WriteBlock(block_info))
                .await
                .unwrap();

            context_svc
                .ready()
                .await
                .unwrap()
                .call(BlockChainContextRequest::Update(info))
                .await
                .unwrap();

            tracing::info!("Verified block: {height}");
        }
    }
}
