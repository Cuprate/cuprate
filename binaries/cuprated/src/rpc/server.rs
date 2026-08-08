//! RPC server initialization and main loop.

use std::{
    cell::{Cell, UnsafeCell},
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Error;
use axum::Router;
use dashmap::DashMap;
use hyper::{body::Incoming, Request};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::{self, auto::Builder as ConnAutoBuilder},
    service::TowerToHyperService,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::{sync::CancellationToken, time::FutureExt};
use tower::{limit::rate::RateLimitLayer, Service};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{error, info, warn};

use cuprate_helper::net::ip_is_local;
use cuprate_rpc_interface::RouterBuilder;

use crate::{
    config::{restricted_rpc_port, unrestricted_rpc_port, RpcConfig},
    rpc::{timeout::WriteTimeout, CupratedRpcHandler},
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

        let rpc_send_timeout = if restricted {
            config.restricted.send_timeout
        } else {
            config.unrestricted.send_timeout
        };

        // Initialize RPC handler service.
        let rpc_handler = CupratedRpcHandler::new(restricted, tx_handler.clone(), launch_ctx);

        // Initialize Axum RPC router.
        let rpc_router = init_rpc_router(rpc_handler, request_byte_limit);

        // Initialize per IP connection limit cache.
        let rpc_limit_cache = RpcLimitCache::from_config(config, restricted);

        // Build the RPC server.
        let rpc_server = RpcServer::new(
            SocketAddr::new(addr, port),
            rpc_send_timeout,
            rpc_limit_cache,
            rpc_router,
        );

        info!(
            restricted,
            address = %addr,
            "Starting RPC server"
        );

        // Launch RPC server.
        let shutdown_token = launch_ctx.task_executor.cancellation_token();
        launch_ctx
            .task_executor
            .spawn(rpc_server.run(shutdown_token));

        info!(restricted, "RPC server shut down.");
    }
}

/// Initialize the Axum router of the RPC server.
fn init_rpc_router(rpc_handler: CupratedRpcHandler, request_byte_limit: usize) -> Router {
    let router = RouterBuilder::new()
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
    let router = if request_byte_limit != 0 {
        router.layer(RequestBodyLimitLayer::new(request_byte_limit))
    } else {
        router
    };

    router.layer(tower_http::trace::TraceLayer::new_for_http())
}

/// An RPC server capable of listening to new connections and serving RPC requests.
pub struct RpcServer {
    /// The RPC Axum router.
    rpc: Router,
    /// The connection limit cache of this server.
    ip_limit_cache: RpcLimitCache,
    listening_address: SocketAddr,
    send_timeout: Duration,
    /// Cancellation token to shutdown this server.
    shutdown_token: Option<CancellationToken>,
}

impl RpcServer {
    /// Build a new RPC server.
    const fn new(
        listening_address: SocketAddr,
        send_timeout: Duration,
        ip_limit_cache: RpcLimitCache,
        rpc: Router,
    ) -> Self {
        Self {
            rpc,
            ip_limit_cache,
            listening_address,
            send_timeout,
            shutdown_token: None,
        }
    }

    /// Consume this server and start serving to incoming connections.
    /// This method only returns errors is unable to listen on its address:port.
    async fn run(mut self, shutdown_token: CancellationToken) -> Result<(), Error> {
        self.shutdown_token = Some(shutdown_token);

        // Start the listener.
        let listener = TcpListener::bind(self.listening_address).await?;

        loop {
            let (socket, remote_addr) = listener.accept().await?;

            // Check for total and per-IP connection limits.
            let Some(token) = self.ip_limit_cache.add_connection(remote_addr) else {
                continue;
            };

            self.serve(socket, remote_addr, token);
        }
    }

    fn serve(&self, socket: TcpStream, remote_addr: SocketAddr, cache: RpcCacheToken) {
        let rpc = self.rpc.clone();
        let socket = WriteTimeout::new(socket, self.send_timeout);

        tokio::spawn(
            async move {
                // Serve RPC router to this connection
                if let Err(err) = ConnAutoBuilder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(
                        TokioIo::new(socket),
                        TowerToHyperService::new(rpc),
                    )
                    .await
                {
                    error!("Failed to serve RPC connection: {err:#}");
                }

                // Clear connection from limit cache
                cache.end_connection();
            }
            .with_cancellation_token_owned(self.shutdown_token.clone().unwrap()),
        );
    }
}

#[derive(Clone)]
/// A concurrently mutated cache of the number of connections of all the
/// IP address currently connected to the RPC server.
pub struct RpcLimitCache {
    /// Maximum amount of connections per public IP address.
    max_conn_count_public_ip: usize,
    /// Maximum amount of connections per private IP address.
    max_conn_count_private_ip: usize,
    /// Maximum amount of connections per loopback IP address.
    max_conn_count_loopback: usize,
    /// Total maximum amount of connections by limited IP addresses.
    max_conn_count: usize,
    /// IP addresses that are excluded from any limitations.
    excluded_ips: Arc<[IpAddr]>,
    /// Amount of current connections per limited IP address.
    per_ip_conn_count: DashMap<SocketAddr, usize>,
    /// Total amount of current connections by limited IP addresses.
    total_conn_count: Arc<AtomicUsize>,
}

impl RpcLimitCache {
    pub fn new(
        max_conn_count_public_ip: usize,
        max_conn_count_private_ip: usize,
        max_conn_count_loopback: usize,
        max_conn_count: usize,
        excluded_ips: Vec<IpAddr>,
    ) -> Self {
        Self {
            max_conn_count_public_ip,
            max_conn_count_private_ip,
            max_conn_count_loopback,
            max_conn_count,
            excluded_ips: Arc::from(excluded_ips),
            total_conn_count: Arc::new(AtomicUsize::new(0)),
            per_ip_conn_count: DashMap::new(),
        }
    }

    /// Initialize an RPC cache from an RPC configuration reference.
    /// (The wrapping sub then + is to convert 0 into MAX)
    pub fn from_config(config: &RpcConfig, restricted: bool) -> Self {
        if restricted {
            Self::new(
                config.restricted.public_ip_connection_limit.wrapping_sub(1) + 1,
                config
                    .restricted
                    .private_ip_connection_limit
                    .wrapping_sub(1)
                    + 1,
                config.restricted.loopback_connection_limit.wrapping_sub(1) + 1,
                config.restricted.total_connection_limit.wrapping_sub(1) + 1,
                config.restricted.excluded_ips_connection_limit.clone(),
            )
        } else {
            Self::new(
                config
                    .unrestricted
                    .public_ip_connection_limit
                    .wrapping_sub(1)
                    + 1,
                config
                    .unrestricted
                    .private_ip_connection_limit
                    .wrapping_sub(1)
                    + 1,
                config
                    .unrestricted
                    .loopback_connection_limit
                    .wrapping_sub(1)
                    + 1,
                config.unrestricted.total_connection_limit.wrapping_sub(1) + 1,
                config.unrestricted.excluded_ips_connection_limit.clone(),
            )
        }
    }

    /// Check the connection limit of the remote IP address, increase it and return a token if allowed,
    /// return nothing if disallowed. Excluded IP address is validated early and do not increase the total
    /// connection count.
    pub fn add_connection(&self, remote_addr: SocketAddr) -> Option<RpcCacheToken> {
        let ip = remote_addr.ip();

        if self.excluded_ips.contains(&ip) {
            return Some(RpcCacheToken::Excluded);
        }

        if let Some(mut conn_count) = self.per_ip_conn_count.get_mut(&remote_addr) {
            let limit = if ip.is_loopback() {
                self.max_conn_count_loopback
            } else if ip_is_local(ip) {
                self.max_conn_count_private_ip
            } else {
                self.max_conn_count_public_ip
            };

            if *conn_count >= limit
                || self.total_conn_count.load(Ordering::Relaxed) >= self.max_conn_count
            {
                return None;
            }

            *conn_count += 1;
        } else {
            self.per_ip_conn_count.insert(remote_addr, 1);
        }
        self.total_conn_count.fetch_add(1, Ordering::AcqRel);

        Some(RpcCacheToken::Limited {
            cache: self.clone(),
            remote_addr,
        })
    }
}

/// Token given to a task to subtract the connection count
/// of the IP address it is serving.
pub enum RpcCacheToken {
    /// This IP address is part of the excluded list and is
    /// not subject to connection count limitations.
    Excluded,
    /// IP address is limited. Keep a reference to the cache
    /// to later subtract its count or remove it.
    Limited {
        cache: RpcLimitCache,
        remote_addr: SocketAddr,
    },
}

impl RpcCacheToken {
    /// Subtract the connection count of and remove if necessary this task's remote address.
    pub fn end_connection(self) {
        if let Self::Limited { cache, remote_addr } = self {
            cache
                .per_ip_conn_count
                .alter(&remote_addr, |_, conn_count| conn_count - 1);
            cache
                .per_ip_conn_count
                .remove_if(&remote_addr, |_, conn_count| *conn_count == 0);
            cache.total_conn_count.fetch_sub(1, Ordering::AcqRel);
        }
    }
}
