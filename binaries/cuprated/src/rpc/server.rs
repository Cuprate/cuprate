//! RPC server initialization and main loop.

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use anyhow::Error;
use tokio::net::TcpListener;
use tower::limit::rate::RateLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{info, warn};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::BlockchainContextService;
use cuprate_rpc_interface::{RouterBuilder, RpcHandler};
use cuprate_txpool::service::TxpoolReadHandle;

use crate::{
    config::{RpcConfig, SharedRpcConfig},
    rpc::{rpc_handler::BlockchainManagerHandle, CupratedRpcHandler},
};

/// Initialize the RPC server(s).
///
/// # Panics
/// This function will panic if:
/// - the server(s) could not be started
/// - unrestricted RPC is started on non-local
///   address without override option
pub fn init_rpc_servers(
    config: RpcConfig,
    blockchain_read: BlockchainReadHandle,
    blockchain_context: BlockchainContextService,
    txpool_read: TxpoolReadHandle,
) {
    for (c, restricted) in [
        (config.unrestricted.shared, false),
        (config.restricted.shared, true),
    ] {
        if !c.enable {
            info!("Skipping RPC server (restricted={restricted})");
            continue;
        };

        let addr = c.address;

        if !restricted && !cuprate_helper::net::ip_is_local(addr.ip()) {
            if config
                .unrestricted
                .i_know_what_im_doing_allow_public_unrestricted_rpc
            {
                warn!(
                    "Starting unrestricted RPC on non-local address ({addr}), this is dangerous!"
                );
            } else {
                panic!("Refusing to start unrestricted RPC on a non-local address ({addr})");
            }
        }

        let rpc_handler = CupratedRpcHandler::new(
            restricted,
            blockchain_read.clone(),
            blockchain_context.clone(),
            txpool_read.clone(),
        );

        tokio::task::spawn(async move {
            run_rpc_server(c, rpc_handler).await.unwrap();
        });
    }
}

/// This initializes and runs an RPC server.
///
/// The function will only return when the server itself returns or an error occurs.
async fn run_rpc_server(
    config: SharedRpcConfig,
    rpc_handler: CupratedRpcHandler,
) -> Result<(), Error> {
    let addr = config.address;

    info!(
        "Starting RPC server (restricted={}) on {addr}",
        rpc_handler.is_restricted()
    );

    // TODO:
    // - add functions that are `all()` but for restricted RPC
    // - enable aliases automatically `other_get_height` + `other_getheight`?
    //
    // FIXME:
    // - `json_rpc` is 1 endpoint; `RouterBuilder` operates at the
    //   level endpoint; we can't selectively enable certain `json_rpc` methods
    let router = RouterBuilder::new()
        .fallback()
        .build()
        .with_state(rpc_handler);

    // Add restrictive layers if restricted RPC.
    //
    // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    let router = if config.request_byte_limit != 0 {
        router.layer(RequestBodyLimitLayer::new(config.request_byte_limit))
    } else {
        router
    };

    // Start the server.
    //
    // TODO: impl custom server code, don't use axum.
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
