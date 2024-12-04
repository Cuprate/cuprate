#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    unused_imports,
    unreachable_pub,
    unreachable_code,
    unused_crate_dependencies,
    dead_code,
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::unused_async,
    clippy::diverging_sub_expression,
    unused_mut,
    clippy::let_unit_value,
    clippy::needless_pass_by_ref_mut,
    reason = "TODO: remove after v1.0.0"
)]

use crate::commands::Command;
use crate::config::Config;
use cuprate_helper::time::secs_to_hms;
use tokio::sync::mpsc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

mod blockchain;
mod commands;
mod config;
mod constants;
mod logging;
mod p2p;
mod rpc;
mod signals;
mod statics;
mod txpool;

fn main() {
    // Initialize global static `LazyLock` data.
    statics::init_lazylock_statics();

    let config = config::read_config_and_args();

    logging::init_logging(&config);

    let rt = init_tokio_rt();

    let db_thread_pool = cuprate_database_service::init_thread_pool(cuprate_database_service::ReaderThreads::Percent(0.3));

    let (mut blockchain_read_handle, mut blockchain_write_handle, _) =
        cuprate_blockchain::service::init_with_pool(config.blockchain_config(), db_thread_pool.clone()).unwrap();
    let (txpool_read_handle, txpool_write_handle, _) =
        cuprate_txpool::service::init_with_pool(config.txpool_config(), db_thread_pool).unwrap();

    rt.block_on(async move {
        blockchain::check_add_genesis(
            &mut blockchain_read_handle,
            &mut blockchain_write_handle,
            config.network(),
        )
        .await;

        let (block_verifier, tx_verifier, context_svc) =
            blockchain::init_consensus(blockchain_read_handle.clone(), config.context_config())
                .await
                .unwrap();

        let (clearnet, incoming_tx_handler_tx) = p2p::start_clearnet_p2p(
            blockchain_read_handle.clone(),
            context_svc.clone(),
            txpool_read_handle.clone(),
            config.clearnet_p2p_config(),
        )
        .await
        .unwrap();

        let tx_handler = txpool::IncomingTxHandler::init(
            clearnet.clone(),
            txpool_write_handle.clone(),
            txpool_read_handle,
            context_svc.clone(),
            tx_verifier,
        );
        if incoming_tx_handler_tx.send(tx_handler).is_err() {
            unreachable!()
        }

        blockchain::init_blockchain_manager(
            clearnet,
            blockchain_write_handle,
            blockchain_read_handle,
            txpool_write_handle,
            context_svc,
            block_verifier,
            config.block_downloader_config(),
        )
        .await;

        let (command_tx, command_rx) = mpsc::channel(1);
        std::thread::spawn(|| commands::command_listener(command_tx));

        io_loop(command_rx).await;
    });
}

fn init_tokio_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(13)
        .enable_all()
        .build()
        .unwrap()
}

async fn io_loop(mut incoming_commands: mpsc::Receiver<Command>) -> ! {
    while let Some(command) = incoming_commands.recv().await {
        match command {
            Command::SetLog { level } => {
                logging::modify_stdout_output(|filter| filter.level = level);

                println!("LOG LEVEL CHANGED: {level}");
            }
            Command::Status => {
                let uptime = statics::START_INSTANT.elapsed().unwrap_or_default();
                let (hours, minutes, second) = secs_to_hms(uptime.as_secs());

                println!("STATUS:\n  uptime: {hours}h {minutes}m {second}s");
            }
        }
    }

    unreachable!()
}
