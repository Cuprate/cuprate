//! RPC server initialization and main loop.

use std::{
    cell::{Cell, UnsafeCell},
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Error;
use axum::Router;
use hyper::{body::Incoming, Request};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::{self, auto::Builder as ConnAutoBuilder},
    service::TowerToHyperService,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinSet,
};
use tokio_util::{sync::CancellationToken, time::FutureExt};
use tower::{limit::rate::RateLimitLayer, Service};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{error, info, warn};

use cuprate_helper::net::ip_is_local;
use cuprate_rpc_interface::RouterBuilder;

use crate::{
    config::{restricted_rpc_port, unrestricted_rpc_port, RpcConfig},
    rpc::{timeout::StreamTimeout, CupratedRpcHandler},
    txpool::IncomingTxHandler,
    LaunchContext,
};

/// Initialize the RPC server(s).
///
/// # Panics
/// This function will panic if:
/// - the server(s) could not be started
/// - unrestricted RPC is started on non-local
///   address without override option
pub fn init_rpc_servers(launch_ctx: &LaunchContext, tx_handler: IncomingTxHandler) {
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
                panic!("Refusing to start unrestricted RPC on a non-local address ({addr})");
            }
        }

        let (rpc_send_timeout, rpc_read_timeout) = if restricted {
            (
                config.restricted.send_timeout,
                config.restricted.read_timeout,
            )
        } else {
            (
                config.unrestricted.send_timeout,
                config.unrestricted.read_timeout,
            )
        };

        // Initialize RPC handler service.
        let rpc_handler = CupratedRpcHandler::new(restricted, tx_handler.clone(), launch_ctx);

        // Initialize Axum RPC router.
        let rpc_router = init_rpc_router(rpc_handler, request_byte_limit);

        // Build the RPC server.
        let rpc_server = RpcServer::new(
            SocketAddr::new(addr, port),
            rpc_send_timeout,
            rpc_read_timeout,
            rpc_router,
        );

        info!(
            restricted,
            address = %addr,
            "Starting RPC server"
        );

        // Launch RPC server.
        let shutdown_token = launch_ctx.task_executor.cancellation_token();
        launch_ctx.task_executor.spawn(async move {
            if let Err(e) = rpc_server.run(shutdown_token).await {
                error!(restricted, "RPC server error: {e:#}");
            }

            info!(restricted, "RPC server shut down.");
        });
    }
}

/// Initialize the Axum router of the RPC server.
fn init_rpc_router(rpc_handler: CupratedRpcHandler, request_byte_limit: usize) -> Router {
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

    router.layer(tower_http::trace::TraceLayer::new_for_http())
}

/// An RPC server capable of listening to new connections and serving RPC requests.
struct RpcServer {
    /// The RPC Axum router.
    rpc: Router,
    listening_address: SocketAddr,
    // Socket timeouts
    send_timeout: Duration,
    read_timeout: Duration,
    rpc_tasks: JoinSet<SocketAddr>,
}

impl RpcServer {
    /// Build a new RPC server.
    fn new(
        listening_address: SocketAddr,
        send_timeout: Duration,
        read_timeout: Duration,
        rpc: Router,
    ) -> Self {
        Self {
            rpc,
            listening_address,
            send_timeout,
            read_timeout,
            rpc_tasks: JoinSet::new(),
        }
    }

    /// Consume this server and start serving to incoming connections.
    /// This method only returns errors is unable to listen on its address:port.
    async fn run(mut self, shutdown_token: CancellationToken) -> Result<(), Error> {
        // Start the listener.
        let listener = TcpListener::bind(self.listening_address).await?;

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

                    self.serve(socket, remote_addr);
                },
                Some(res) = self.rpc_tasks.join_next() => {
                    let res = res?;
                }
                () = shutdown_token.cancelled() => {
                    self.rpc_tasks.abort_all();

                    return Ok(());
                }
            }
        }
    }

    fn serve(&mut self, socket: TcpStream, remote_addr: SocketAddr) {
        let rpc = self.rpc.clone();
        let socket = StreamTimeout::new(socket, self.send_timeout, self.read_timeout);

        self.rpc_tasks.spawn(async move {
            // Serve RPC router to this connection
            if let Err(err) = ConnAutoBuilder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(socket), TowerToHyperService::new(rpc))
                .await
            {
                error!("Failed to serve RPC connection: {err:#}");
            }

            remote_addr
        });
    }
}
