//! RPC server init.

use std::{net::SocketAddr, time::Duration};

use anyhow::Error;
use tokio::net::TcpListener;
use tower::limit::rate::RateLimitLayer;
use tower_http::{
    compression::CompressionLayer, decompression::DecompressionLayer, limit::RequestBodyLimitLayer,
};
use tracing::info;

use cuprate_rpc_interface::{RouterBuilder, RpcHandlerDummy};

use crate::{config::RpcConfig, rpc::CupratedRpcHandler};

/// Initialize the RPC server(s).
///
/// # Panics
/// This function will panic if the server(s) could not be started.
pub fn init_rpc_servers(config: RpcConfig) {
    for (option, restricted) in [
        (Some(config.address), false),
        (config.address_restricted, true),
    ] {
        let Some(socket_addr) = option else {
            continue;
        };

        let conf = config.clone();

        tokio::task::spawn(async move {
            init_rpc_server(socket_addr, restricted, conf)
                .await
                .unwrap();
        });
    }
}

async fn init_rpc_server(
    socket_addr: SocketAddr,
    restricted: bool,
    config: RpcConfig,
) -> Result<(), Error> {
    info!("Starting RPC server (restricted={restricted}) on {socket_addr}");

    // Create the router.
    //
    // TODO: impl more layers, rate-limiting, configuration, etc.
    let state = RpcHandlerDummy { restricted };
    let router = RouterBuilder::new().all().build().with_state(state);

    // Enable request (de)compression.
    let router = if config.gzip || config.br {
        router
            .layer(DecompressionLayer::new().gzip(config.gzip).br(config.br))
            .layer(CompressionLayer::new().gzip(config.gzip).br(config.br))
    } else {
        router
    };

    // Add restrictive layers if restricted RPC.
    //
    // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    let router = if restricted {
        router.layer(RequestBodyLimitLayer::new(
            config.restricted_request_byte_limit,
        ))
    } else {
        router
    };

    // Start the server.
    let listener = TcpListener::bind(socket_addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
