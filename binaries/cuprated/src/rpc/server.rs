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
use tracing::{error, info, warn};

use cuprate_rpc_interface::{RouterBuilder, RpcHandler};

use crate::{
    config::{restricted_rpc_port, unrestricted_rpc_port},
    rpc::CupratedRpcHandler,
    txpool::IncomingTxHandler,
    LaunchContext,
};

/// Initialize the RPC server(s).
///
/// # Errors
///
/// This function will return an [`Err`] if unrestricted RPC is started on a
/// non-local address without the override option, or if an RPC listener cannot
/// be bound.
pub async fn init_rpc_servers(
    launch_ctx: &LaunchContext,
    tx_handler: IncomingTxHandler,
) -> Result<(), Error> {
    let config = &launch_ctx.config.rpc;
    for ((enable, addr, port, request_byte_limit), restricted) in [
        (
            (
                config.unrestricted.enable,
                config.unrestricted.address,
                unrestricted_rpc_port(config.unrestricted.port, launch_ctx.config.network),
                config.unrestricted.request_byte_limit,
            ),
            false,
        ),
        (
            (
                config.restricted.enable,
                config.restricted.address,
                restricted_rpc_port(config.restricted.port, launch_ctx.config.network),
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
                anyhow::bail!("Refusing to start unrestricted RPC on a non-local address ({addr})");
            }
        }

        let rpc_handler = CupratedRpcHandler::new(restricted, tx_handler.clone(), launch_ctx);
        let address = SocketAddr::new(addr, port);
        let listener = TcpListener::bind(address).await?;

        let shutdown_token = launch_ctx.task_executor.cancellation_token();
        launch_ctx.task_executor.spawn(async move {
            if let Err(e) = run_rpc_server(
                rpc_handler,
                restricted,
                address,
                listener,
                request_byte_limit,
                shutdown_token,
            )
            .await
            {
                error!(restricted, "Failed to start RPC server: {e:#}");
            }
        });
    }

    Ok(())
}

/// This initializes and runs an RPC server.
///
/// The function will only return when the server itself returns or an error occurs.
async fn run_rpc_server(
    rpc_handler: CupratedRpcHandler,
    restricted: bool,
    address: SocketAddr,
    listener: TcpListener,
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
    let router = RouterBuilder::new().all().build().with_state(rpc_handler);

    // Add restrictive layers if restricted RPC.
    //
    // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    let router = if request_byte_limit != 0 {
        router.layer(RequestBodyLimitLayer::new(request_byte_limit))
    } else {
        router
    };

    let router = router.layer(tower_http::trace::TraceLayer::new_for_http());

    // Start the server.
    //
    // TODO: impl custom server code, don't use axum.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_token.cancelled_owned())
        .await?;

    info!(restricted, "RPC server shut down.");
    Ok(())
}
