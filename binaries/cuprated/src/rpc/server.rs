//! RPC server init.

use std::net::SocketAddr;

use anyhow::Error;
use tokio::net::TcpListener;
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

        tokio::task::spawn(async move {
            init_rpc_server(socket_addr, restricted).await.unwrap();
        });
    }
}

async fn init_rpc_server(socket_addr: SocketAddr, restricted: bool) -> Result<(), Error> {
    info!("Starting RPC server (restricted={restricted}) on {socket_addr}");

    // Create the router.
    let state = RpcHandlerDummy { restricted };
    let router = RouterBuilder::new().all().build().with_state(state);

    // Start a server.
    let listener = TcpListener::bind(socket_addr).await?;

    // Run the server with `axum`.
    //
    // TODO: impl layers, rate-limiting, configuration, etc.
    axum::serve(listener, router).await?;

    Ok(())
}
