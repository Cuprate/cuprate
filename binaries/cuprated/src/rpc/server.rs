//! RPC server initialization and main loop.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use anyhow::Error;
use axum::Router;
use dashmap::{DashMap, DashSet};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as ConnAutoBuilder,
    service::TowerToHyperService,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::{AbortHandle, JoinSet},
};
use tokio_util::{sync::CancellationToken, time::FutureExt};
use tower::Service;
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

        // Initialize per IP connection limit cache.
        let rpc_limit_cache = RpcLimitCache::from_config(config, restricted);

        // Build the RPC server.
        let rpc_server = RpcServer::new(
            SocketAddr::new(addr, port),
            rpc_send_timeout,
            rpc_read_timeout,
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
    /// The connection limit cache of this server.
    ip_limit_cache: RpcLimitCache,
    listening_address: SocketAddr,
    // Socket timeouts
    send_timeout: Duration,
    read_timeout: Duration,
    rpc_tasks: JoinSet<(SocketAddr, bool)>,
}

impl RpcServer {
    /// Build a new RPC server.
    fn new(
        listening_address: SocketAddr,
        send_timeout: Duration,
        read_timeout: Duration,
        ip_limit_cache: RpcLimitCache,
        rpc: Router,
    ) -> Self {
        Self {
            rpc,
            ip_limit_cache,
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

                    // Check if IP is banned
                    if self.ip_limit_cache.is_ip_banned(remote_addr.ip()) {
                        continue;
                    }

                    // Check for total and per-IP connection limits.
                    if !self.ip_limit_cache.check_and_track_connection(remote_addr.ip()) {
                        continue;
                    }

                    self.serve(socket, remote_addr);
                },
                Some(res) = self.rpc_tasks.join_next() => {
                    let (addr, serve_failed) = res?;
                    let ip = addr.ip();

                    // Untrack connection from IP
                    self.ip_limit_cache.remove_connection(&ip);

                    // If hyper serve failed, increase the serve failure and ban the IP
                    // for 3 seconds if it reaches 15 failures.
                    if serve_failed && self.ip_limit_cache.increment_serve_failure(&ip) >= 15 {
                        self.ip_limit_cache.ban_serve_ip(&ip);
                    }
                }
                () = shutdown_token.cancelled() => {
                    self.rpc_tasks.abort_all();

                    return Ok(());
                }
            }
        }
    }

    fn serve(&mut self, socket: TcpStream, remote_addr: SocketAddr) {
        let mut make_service = self
            .rpc
            .clone()
            .into_make_service_with_connect_info::<SocketAddr>();
        let socket = StreamTimeout::new(socket, self.send_timeout, self.read_timeout);

        self.rpc_tasks.spawn(async move {
            let Ok(rpc) = make_service.call(remote_addr).await;

            // Serve RPC router to this connection
            if let Err(err) = ConnAutoBuilder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(socket), TowerToHyperService::new(rpc))
                .await
            {
                error!("Failed to serve RPC connection: {err:#}");
                return (remote_addr, true);
            }

            (remote_addr, false)
        });
    }
}

/// A cache of the number of connections of all the
/// IP addresses currently connected to an RPC server.
struct RpcLimitCache {
    /// Maximum amount of connections per public IP address.
    max_conn_count_public_ip: Option<NonZeroUsize>,
    /// Maximum amount of connections per private IP address.
    max_conn_count_private_ip: Option<NonZeroUsize>,
    /// Maximum amount of connections per loopback IP address.
    max_conn_count_loopback: Option<NonZeroUsize>,
    /// Total maximum amount of connections by limited IP addresses.
    max_conn_count: Option<NonZeroUsize>,
    /// IP addresses that are excluded from any limitations.
    excluded_ips: Vec<IpAddr>,
    /// Amount of current connections per limited IP address.
    per_ip_conn_count: HashMap<IpAddr, usize>,
    /// Total amount of current connections by limited IP addresses.
    total_conn_count: usize,
    /// IP addresses banned after failing HTTP stack too many times.
    banned_ips: Arc<DashMap<IpAddr, usize>>,
}

impl RpcLimitCache {
    /// Initialize an RPC cache from an RPC configuration reference.
    fn from_config(config: &RpcConfig, restricted: bool) -> Self {
        if restricted {
            Self {
                max_conn_count_public_ip: NonZeroUsize::new(
                    config.restricted.public_ip_connection_limit,
                ),
                max_conn_count_private_ip: NonZeroUsize::new(
                    config.restricted.private_ip_connection_limit,
                ),
                max_conn_count_loopback: NonZeroUsize::new(
                    config.restricted.loopback_connection_limit,
                ),
                max_conn_count: NonZeroUsize::new(config.restricted.total_connection_limit),
                excluded_ips: config.restricted.excluded_ips_connection_limit.clone(),
                total_conn_count: 0,
                per_ip_conn_count: HashMap::new(),
                banned_ips: Arc::new(DashMap::new()),
            }
        } else {
            Self {
                max_conn_count_public_ip: NonZeroUsize::new(
                    config.unrestricted.public_ip_connection_limit,
                ),
                max_conn_count_private_ip: NonZeroUsize::new(
                    config.unrestricted.private_ip_connection_limit,
                ),
                max_conn_count_loopback: NonZeroUsize::new(
                    config.unrestricted.loopback_connection_limit,
                ),
                max_conn_count: NonZeroUsize::new(config.unrestricted.total_connection_limit),
                excluded_ips: config.unrestricted.excluded_ips_connection_limit.clone(),
                total_conn_count: 0,
                per_ip_conn_count: HashMap::new(),
                banned_ips: Arc::new(DashMap::new()),
            }
        }
    }

    /// Check the connection limit of the remote IP address, increase it and return `true` if allowed,
    /// return `false` if disallowed. Excluded IP address is validated early and do not increase the total
    /// connection count.
    fn check_and_track_connection(&mut self, remote_addr: IpAddr) -> bool {
        if self.excluded_ips.contains(&remote_addr) {
            return true;
        }

        if let Some(max_conn_count) = self.max_conn_count {
            if self.total_conn_count >= max_conn_count.get() {
                return false;
            }
        }

        if let Some(mut conn_count) = self.per_ip_conn_count.get_mut(&remote_addr) {
            let limit = if remote_addr.is_loopback() {
                self.max_conn_count_loopback
            } else if ip_is_local(remote_addr) {
                self.max_conn_count_private_ip
            } else {
                self.max_conn_count_public_ip
            };

            if let Some(limit) = limit {
                if *conn_count >= limit.get() {
                    return false;
                }
            }

            *conn_count += 1;
        } else {
            self.per_ip_conn_count.insert(remote_addr, 1);
        }

        self.total_conn_count += 1;

        true
    }

    /// Remove a connection of the remote IP address.
    fn remove_connection(&mut self, remote_addr: &IpAddr) {
        if self.excluded_ips.contains(remote_addr) {
            return;
        }

        let Some(value) = self.per_ip_conn_count.get_mut(remote_addr) else {
            return;
        };

        *value = value.saturating_sub(1);
        self.total_conn_count -= 1;

        if *value == 0 {
            self.per_ip_conn_count.remove(remote_addr);
        }
    }

    fn is_ip_banned(&mut self, remote: IpAddr) -> bool {
        if let Some(counter) = self.banned_ips.get(&remote) {
            *counter >= 15
        } else {
            false
        }
    }

    /// Increment the failure counter for an IP address and returning
    /// its new count. This creates a new entry if the IP address has never
    /// experienced any failure and isn't present in cache.
    fn increment_serve_failure(&mut self, remote: &IpAddr) -> usize {
        if let Some(mut counter) = self.banned_ips.get_mut(remote) {
            *counter += 1;
            *counter
        } else {
            self.banned_ips.insert(*remote, 1);
            1
        }
    }

    /// Ban a specific IP from being served for 3 seconds.
    /// After this delay, this delay its entry is deleted from
    /// the banning cache.
    fn ban_serve_ip(&mut self, remote: &IpAddr) {
        let banned_ips = Arc::clone(&self.banned_ips);
        let remote = *remote;

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            banned_ips.remove(&remote);
        });
    }
}
