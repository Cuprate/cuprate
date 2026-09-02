//! RPC server initialization and main loop.

use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Error};
use axum::Router;
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinSet,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use tower_http::{limit::RequestBodyLimitLayer, timeout::RequestBodyDeadlineLayer};
use tracing::{debug, info, warn};

use cuprate_helper::net::ip_is_local;
use cuprate_rpc_interface::RouterBuilder;

use crate::{
    config::{restricted_rpc_port, unrestricted_rpc_port},
    rpc::{timeout::WriteTimeout, CupratedRpcHandler},
    txpool::IncomingTxHandler,
    LaunchContext,
};

/// The maximum amount of time we wait for connections to gracefully close
/// after shutdown signal.
const RPC_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Initialize the RPC server(s).
///
/// # Errors
///
/// This function will return an [`Err`] if unrestricted RPC is started on a
/// non-local address without the override option, or if an RPC listener cannot
/// be bound.
pub(crate) async fn init_rpc_servers(
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

        if !restricted && !ip_is_local(addr) {
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

        let (rpc_send_timeout, rpc_header_read_timeout, rpc_body_read_timeout) = if restricted {
            (
                config.restricted.send_timeout,
                config.restricted.header_read_timeout,
                config.restricted.body_read_timeout,
            )
        } else {
            (
                config.unrestricted.send_timeout,
                config.unrestricted.header_read_timeout,
                config.unrestricted.body_read_timeout,
            )
        };

        // Initialize RPC handler service.
        let rpc_handler = CupratedRpcHandler::new(restricted, tx_handler.clone(), launch_ctx);
        let address = SocketAddr::new(addr, port);
        let listener = TcpListener::bind(address)
            .await
            .with_context(|| format!("failed to bind RPC listener on {address}"))?;

        // Initialize Axum RPC router.
        let rpc_router = init_rpc_router(rpc_handler, request_byte_limit, rpc_body_read_timeout);

        // Build the RPC server.
        let rpc_server = RpcServer::new(rpc_send_timeout, rpc_header_read_timeout, rpc_router);

        info!(
            restricted,
            address = %address,
            "Starting RPC server"
        );

        // Launch RPC server.
        let shutdown_token = launch_ctx.task_executor.cancellation_token();
        launch_ctx.task_executor.spawn(async move {
            rpc_server.run(listener, shutdown_token).await;

            info!(restricted, "RPC server shut down.");
        });
    }

    Ok(())
}

/// Initialize the Axum router of the RPC server.
fn init_rpc_router(
    rpc_handler: CupratedRpcHandler,
    request_byte_limit: usize,
    body_read_timeout: Duration,
) -> Router {
    let mut router = RouterBuilder::new()
        .json_rpc()
        //
        .other_get_info()
        .other_getinfo()
        .other_get_height()
        .other_getheight()
        .other_get_transactions()
        .other_gettransactions()
        .other_is_key_image_spent()
        .other_send_raw_transaction()
        .other_sendrawtransaction()
        .other_get_transaction_pool_hashes()
        .other_stop_daemon()
        //
        .bin_get_blocks()
        .bin_getblocks()
        .bin_get_blocks_by_height()
        .bin_getblocks_by_height()
        .bin_get_hashes()
        .bin_gethashes()
        .bin_get_o_indexes()
        .bin_get_outs()
        .bin_get_transaction_pool_hashes()
        .bin_get_output_distribution()
        //
        .fallback()
        .build()
        .with_state(rpc_handler);

    // Add restrictive layers if restricted RPC.
    //
    // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    if request_byte_limit != 0 {
        router = router.layer(RequestBodyLimitLayer::new(request_byte_limit));
    }

    router
        .layer(RequestBodyDeadlineLayer::new(body_read_timeout))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

/// An RPC server capable of listening to new connections and serving RPC requests.
struct RpcServer {
    /// The RPC Axum router.
    rpc: Router,
    // Socket timeouts
    send_timeout: Duration,
    header_read_timeout: Duration,
    rpc_tasks: JoinSet<SocketAddr>,
}

impl RpcServer {
    /// Build a new RPC server.
    fn new(send_timeout: Duration, header_read_timeout: Duration, rpc: Router) -> Self {
        Self {
            rpc,
            send_timeout,
            header_read_timeout,
            rpc_tasks: JoinSet::new(),
        }
    }

    /// Consume this server and start serving connections incoming on `listener`.
    async fn run(mut self, listener: TcpListener, shutdown_token: CancellationToken) {
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (socket, remote_addr) = match res {
                        Ok(res) => res,
                        Err(e) => {
                            // A failed accept can be caused by reaching resource limits.
                            // We should not attempt to accept a new connection immediately.
                            warn!("Failed to accept RPC connection: {e}");
                            tokio::time::sleep(Duration::from_millis(40)).await;
                            continue
                        }
                    };

                    self.serve(socket, remote_addr, shutdown_token.clone());
                },
                Some(Err(err)) = self.rpc_tasks.join_next() => {
                    debug!("RPC serving task failed: {err:#}");
                }
                () = shutdown_token.cancelled() => {
                    break;
                }
            }
        }

        // Graceful shutdown of all connections for up to 5 seconds.
        if timeout(RPC_SHUTDOWN_TIMEOUT, async move {
            while let Some(res) = self.rpc_tasks.join_next().await {
                if let Err(err) = res {
                    warn!("RPC serving task failed: {err:#}");
                }
            }
        })
        .await
        .is_err()
        {
            warn!("RPC tasks survived shutdown signal for more than {} seconds... Dropping connections anyway.", RPC_SHUTDOWN_TIMEOUT.as_secs());
        }
    }

    fn serve(
        &mut self,
        socket: TcpStream,
        remote_addr: SocketAddr,
        shutdown_token: CancellationToken,
    ) {
        let rpc = self.rpc.clone();
        let send_timeout = self.send_timeout;
        let header_read_timeout = self.header_read_timeout;

        self.rpc_tasks.spawn(async move {
            let socket = WriteTimeout::new(socket, send_timeout);

            let mut builder = http1::Builder::new();
            builder
                .timer(TokioTimer::new())
                .header_read_timeout(header_read_timeout);

            let connection =
                builder.serve_connection(TokioIo::new(socket), TowerToHyperService::new(rpc));
            tokio::pin!(connection);

            tokio::select! {
                result = &mut connection => {
                    if let Err(err) = result {
                        debug!("Failed to serve RPC connection: {err:#}");
                    }
                }
                () = shutdown_token.cancelled() => {
                    connection.as_mut().graceful_shutdown();

                    if let Err(err) = connection.await {
                        debug!("Failed to shut down RPC connection: {err:#}");
                    }
                }
            }

            remote_addr
        });
    }
}
