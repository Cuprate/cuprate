use tokio::runtime::Runtime;
use tracing::Level;

mod blockchain;
mod config;
mod p2p;
mod rpc;
mod txpool;

use blockchain::check_add_genesis;
use config::CupratedConfig;

fn main() {
    let config = config::config();

    init_log(&config);

    let (mut bc_read_handle, mut bc_write_handle, _) =
        cuprate_blockchain::service::init(config.blockchain_config()).unwrap();

    let async_rt = init_tokio_rt(&config);

    async_rt.block_on(async move {
        check_add_genesis(&mut bc_read_handle, &mut bc_write_handle, &config.network()).await;

        let (block_verifier, _tx_verifier, context_svc) =
            blockchain::init_consensus(bc_read_handle.clone(), config.context_config())
                .await
                .unwrap();

        let net = cuprate_p2p::initialize_network(
            p2p::request_handler::P2pProtocolRequestHandler,
            p2p::core_sync_svc::CoreSyncService(context_svc.clone()),
            config.clearnet_config(),
        )
        .await
        .unwrap();

        blockchain::init_blockchain_manager(
            net,
            bc_write_handle,
            bc_read_handle,
            context_svc,
            block_verifier,
            config.block_downloader_config(),
        );

        // TODO: this can be removed as long as the main thread does not exit, so when command handling
        // is added
        futures::future::pending::<()>().await;
    });

    // TODO: add command handling.
}

fn init_log(_config: &CupratedConfig) {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
}

fn init_tokio_rt(_config: &CupratedConfig) -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}
