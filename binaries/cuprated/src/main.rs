use crate::blockchain::check_add_genesis;
use clap::Parser;
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p::P2PConfig;
use cuprate_p2p_core::Network;
use std::time::Duration;
use tracing::Level;

mod blockchain;
mod config;
mod p2p;
mod rpc;
mod txpool;

#[derive(Parser)]
struct Args {}
fn main() {
    let _args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let (mut bc_read_handle, mut bc_write_handle, _) =
        cuprate_blockchain::service::init(cuprate_blockchain::config::Config::default()).unwrap();

    let async_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    async_rt.block_on(async move {
        check_add_genesis(&mut bc_read_handle, &mut bc_write_handle, &Network::Mainnet).await;

        let (block_verifier, _tx_verifier, context_svc) = blockchain::init_consensus(
            bc_read_handle.clone(),
            cuprate_consensus::ContextConfig::main_net(),
        )
        .await
        .unwrap();

        let net = cuprate_p2p::initialize_network(
            p2p::request_handler::P2pProtocolRequestHandler,
            p2p::core_sync_svc::CoreSyncService(context_svc.clone()),
            p2p::dummy_config(),
        )
        .await
        .unwrap();

        blockchain::init_blockchain_manager(
            net,
            BlockDownloaderConfig {
                buffer_size: 50_000_000,
                in_progress_queue_size: 50_000_000,
                check_client_pool_interval: Duration::from_secs(45),
                target_batch_size: 10_000_000,
                initial_batch_size: 1,
            },
            bc_write_handle,
            bc_read_handle,
            context_svc,
            block_verifier,
        );

        tokio::time::sleep(Duration::MAX).await;
    });
}
