//! RPC server initialization and main loop.

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use anyhow::Error;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower::limit::rate::RateLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{info, warn};

use cuprate_rpc_interface::{RouterBuilder, RpcHandler};

use crate::{
    config::{restricted_rpc_port, unrestricted_rpc_port, RpcConfig},
    rpc::CupratedRpcHandler,
    txpool::IncomingTxHandler,
    NodeContext,
};

/// Initialize the RPC server(s).
///
/// # Panics
/// This function will panic if:
/// - the server(s) could not be started
/// - unrestricted RPC is started on non-local
///   address without override option
pub fn init_rpc_servers(config: RpcConfig, tx_handler: IncomingTxHandler, node_ctx: NodeContext) {
    for ((enable, addr, port, request_byte_limit), restricted) in [
        (
            (
                config.unrestricted.enable,
                config.unrestricted.address,
                unrestricted_rpc_port(config.unrestricted.port, node_ctx.network),
                config.unrestricted.request_byte_limit,
            ),
            false,
        ),
        (
            (
                config.restricted.enable,
                config.restricted.address,
                restricted_rpc_port(config.restricted.port, node_ctx.network),
                config.restricted.request_byte_limit,
            ),
            true,
        ),
    ] {
        if !enable {
            info!(restricted, "Skipping RPC server");
            continue;
        }

        if !restricted && !cuprate_helper::net::ip_is_local(addr) {
            if config
                .unrestricted
                .i_know_what_im_doing_allow_public_unrestricted_rpc
            {
                warn!(
                    address = %addr,
                    "Starting unrestricted RPC on non-local address, this is dangerous!"
                );
            } else {
                panic!("Refusing to start unrestricted RPC on a non-local address ({addr})");
            }
        }

        let rpc_handler = CupratedRpcHandler::new(restricted, tx_handler.clone(), node_ctx.clone());

        let shutdown_token = node_ctx.task_executor.cancellation_token().clone();
        node_ctx.task_executor.spawn_critical(async move {
            run_rpc_server(
                rpc_handler,
                restricted,
                SocketAddr::new(addr, port),
                request_byte_limit,
                shutdown_token,
            )
            .await?;
            info!(restricted, "RPC server shut down.");
            Ok(())
        });
    }
}

/// This initializes and runs an RPC server.
///
/// The function will only return when the server itself returns or an error occurs.
async fn run_rpc_server(
    rpc_handler: CupratedRpcHandler,
    restricted: bool,
    address: SocketAddr,
    request_byte_limit: usize,
    shutdown_token: CancellationToken,
) -> Result<(), Error> {
    info!(
        restricted,
        address = %address,
        "Starting RPC server"
    );

    // TODO:
    // - add functions that are `all()` but for restricted RPC
    // - enable aliases automatically `other_get_height` + `other_getheight`?
    let router = RouterBuilder::new()
        .json_rpc()
        .other_get_height()
        .other_send_raw_transaction()
        .other_sendrawtransaction()
        .fallback()
        .build()
        .with_state(rpc_handler);

    // Add restrictive layers if restricted RPC.
    //
    // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    let router = if request_byte_limit != 0 {
        router.layer(RequestBodyLimitLayer::new(request_byte_limit))
    } else {
        router
    };

    // Start the server.
    //
    // TODO: impl custom server code, don't use axum.
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_token.cancelled_owned())
        .await?;

    Ok(())
}
