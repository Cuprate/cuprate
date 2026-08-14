//! RPC server initialization and main loop.

use std::{
    collections::{hash_map::Entry, HashMap},
    net::{IpAddr, SocketAddr},
    num::NonZeroUsize,
    time::Duration,
};

use anyhow::Error;
use axum::Router;
use futures::StreamExt;
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
use tokio_util::{
    sync::CancellationToken,
    time::{delay_queue::Key, DelayQueue},
};
use tower_http::{limit::RequestBodyLimitLayer, timeout::RequestBodyDeadlineLayer};
use tracing::{debug, error, info, warn};

use cuprate_helper::net::ip_is_local;
use cuprate_rpc_interface::RouterBuilder;

use crate::{
    config::{restricted_rpc_port, unrestricted_rpc_port, RpcConfig},
    rpc::{timeout::WriteTimeout, CupratedRpcHandler},
    txpool::IncomingTxHandler,
    LaunchContext,
};

/// The maximum amount of time we wait for connections to gracefully close
/// after shutdown signal.
const RPC_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// The amount of failures to serve in a short period of time we consider
/// worth banning the IP address for.
const RPC_FAILURE_BAN_THRESHOLD: usize = 15;

/// The amount of time we keep IP addresses failure in cache. If no failure
/// has been produced during this period of time, the IP's failures are removed.
const RPC_FAILURE_KEEP_ALIVE: Duration = Duration::from_secs(5);

/// The amount of time banned IP addresses are unable to be served.
const RPC_BAN_PERIOD: Duration = Duration::from_secs(3);

/// Initialize the RPC server(s).
///
/// # Panics
/// This function will panic if:
/// - the server(s) could not be started
/// - unrestricted RPC is started on non-local
///   address without override option
pub(crate) async fn init_rpc_servers(
    launch_ctx: &LaunchContext,
    tx_handler: &IncomingTxHandler,
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
                    port = %port,
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

        // Initialize Axum RPC router.
        let rpc_router = init_rpc_router(rpc_handler, request_byte_limit, rpc_body_read_timeout);

        // Initialize per IP connection limit cache.
        let rpc_limit_cache = RpcLimitCache::from_config(config, restricted);

        // Build the RPC server.
        let rpc_server = RpcServer::new(
            rpc_send_timeout,
            rpc_header_read_timeout,
            rpc_limit_cache,
            rpc_router,
        );

        info!(
            restricted,
            address = %addr,
            "Starting RPC server"
        );

        // Bind RPC server listener.
        let listener = TcpListener::bind(SocketAddr::new(addr, port)).await?;

        // Launch RPC server.
        let shutdown_token = launch_ctx.task_executor.cancellation_token();
        launch_ctx.task_executor.spawn(async move {
            if let Err(e) = rpc_server.run(listener, shutdown_token).await {
                error!(restricted, "RPC server error: {e:#}");
            }

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
    /// The connection limit cache of this server.
    ip_limit_cache: RpcLimitCache,
    // Socket timeouts
    send_timeout: Duration,
    header_read_timeout: Duration,
    rpc_tasks: JoinSet<(SocketAddr, bool)>,
}

impl RpcServer {
    /// Build a new RPC server.
    fn new(
        send_timeout: Duration,
        header_read_timeout: Duration,
        ip_limit_cache: RpcLimitCache,
        rpc: Router,
    ) -> Self {
        Self {
            rpc,
            ip_limit_cache,
            send_timeout,
            header_read_timeout,
            rpc_tasks: JoinSet::new(),
        }
    }

    /// Consume this server and start serving to incoming connections.
    /// This method only returns errors if it is unable to listen on its address:port.
    async fn run(
        mut self,
        listener: TcpListener,
        shutdown_token: CancellationToken,
    ) -> Result<(), Error> {
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

                    let ip = remote_addr.ip().to_canonical();

                    // Excluded IPs bypass ban and connection limits
                    if !self.ip_limit_cache.is_excluded(ip) {
                        // Check if IP is banned and connection limits.
                        if self.ip_limit_cache.is_ip_banned(ip)
                        || !self.ip_limit_cache.check_and_track_connection(ip)
                        {
                            debug!("RPC refused connection from banned ip address: {ip}");
                            continue;
                        }
                    }

                    self.serve(socket, remote_addr, shutdown_token.clone());
                },
                Some(res) = self.rpc_tasks.join_next() => {
                    let (addr, serve_failed) = res?;
                    let ip = addr.ip();

                    if self.ip_limit_cache.is_excluded(ip) {
                        continue;
                    }

                    // Untrack connection from IP
                    self.ip_limit_cache.remove_connection(&ip);

                    // If hyper serve failed, increase the serve failure and ban the IP
                    // for `RPC_BAN_PERIOD` seconds if it reaches `RPC_FAILURE_BAN_THRESHOLD` failures.
                    if serve_failed {
                        self.ip_limit_cache.increment_serve_failure(&ip);
                    }
                }
                Some(expiry) = self.ip_limit_cache.per_ip_failures_expiry.next() => {
                    let ip = expiry.into_inner();
                    if let Some((failures, _)) = self.ip_limit_cache.per_ip_failures.remove(&ip) {
                        if failures >= RPC_FAILURE_BAN_THRESHOLD {
                            // An IP address has been banned for the entire ban period,
                            // lifting its ban.
                            debug!("RPC lifted ban for IP address {}", ip);
                        } else {
                            // An IP address stopped producing failures after a certain period.
                            // Removing its entry from the cache.
                            debug!("RPC evicted {}'s serving failures", ip);
                        }
                    }
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
            warn!("RPC tasks survived shutdown signal for more than {RPC_SHUTDOWN_TIMEOUT:?}... Dropping connections anyway.");
        }

        Ok(())
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
                        return (remote_addr, true);
                    }
                }
                () = shutdown_token.cancelled() => {
                    connection.as_mut().graceful_shutdown();

                    if let Err(err) = connection.await {
                        debug!("Failed to shut down RPC connection: {err:#}");
                        return (remote_addr, true);
                    }
                }
            }

            (remote_addr, false)
        });
    }
}

/// A cache storing the number of connections of all the
/// IP addresses, as well as their respective failure rate and
/// banned state.
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
    /// Amount of serving failures per limited IP address and its expiry key.
    per_ip_failures: HashMap<IpAddr, (usize, Key)>,
    /// The expiry for every entry in `per_ip_failures`.
    per_ip_failures_expiry: DelayQueue<IpAddr>,
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
                per_ip_failures: HashMap::new(),
                per_ip_failures_expiry: DelayQueue::new(),
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
                per_ip_failures: HashMap::new(),
                per_ip_failures_expiry: DelayQueue::new(),
            }
        }
    }

    /// Check the connection limit of the remote IP address, increase it and
    /// return `true` if allowed, return `false` if disallowed.
    ///
    /// Excluded IP addresses must be filtered out by the caller.
    fn check_and_track_connection(&mut self, remote_addr: IpAddr) -> bool {
        if let Some(max_conn_count) = self.max_conn_count {
            if self.total_conn_count >= max_conn_count.get() {
                return false;
            }
        }

        match self.per_ip_conn_count.entry(remote_addr) {
            Entry::Occupied(mut entry) => {
                let limit = if remote_addr.is_loopback() {
                    self.max_conn_count_loopback
                } else if ip_is_local(remote_addr) {
                    self.max_conn_count_private_ip
                } else {
                    self.max_conn_count_public_ip
                };

                if let Some(limit) = limit {
                    if *entry.get() >= limit.get() {
                        return false;
                    }
                }

                *entry.get_mut() += 1;
            }
            Entry::Vacant(entry) => {
                entry.insert(1);
            }
        }

        self.total_conn_count += 1;
        true
    }

    /// Remove a connection of the remote IP address.
    ///
    /// Excluded IP addresses must be filtered out by the caller.
    fn remove_connection(&mut self, remote_addr: &IpAddr) {
        let Entry::Occupied(mut entry) = self.per_ip_conn_count.entry(*remote_addr) else {
            return;
        };

        let value = entry.get_mut();
        *value = value.saturating_sub(1);
        self.total_conn_count -= 1;

        if *value == 0 {
            entry.remove();
        }
    }

    /// Whether this IP is known to have cause failure
    /// and if so, did it caused `RPC_FAILURE_BAN_THRESHOLD` failures.
    fn is_ip_banned(&self, remote: IpAddr) -> bool {
        if let Some(counter) = self.per_ip_failures.get(&remote) {
            counter.0 >= RPC_FAILURE_BAN_THRESHOLD
        } else {
            false
        }
    }

    /// Whether this IP address is excluded from all limits.
    fn is_excluded(&self, remote_addr: IpAddr) -> bool {
        self.excluded_ips.contains(&remote_addr)
    }

    /// Increment the failure counter for an IP address and ban it
    /// if it reaches `RPC_FAILURE_BAN_THRESHOLD`.
    ///
    /// This creates a new entry if the IP address has never
    /// experienced any failure and isn't present in cache.
    fn increment_serve_failure(&mut self, remote: &IpAddr) {
        match self.per_ip_failures.entry(*remote) {
            Entry::Vacant(entry) => {
                let expiry_key = self
                    .per_ip_failures_expiry
                    .insert(*remote, RPC_FAILURE_KEEP_ALIVE);
                entry.insert((1, expiry_key));
            }
            Entry::Occupied(mut entry) => {
                let (counter, expiry_key) = entry.get_mut();

                // Failures while banned neither increase the counter nor
                // extend the ban period.
                if *counter >= RPC_FAILURE_BAN_THRESHOLD {
                    return;
                }

                *counter += 1;
                let expiry = if *counter >= RPC_FAILURE_BAN_THRESHOLD {
                    debug!("RPC banned IP address {remote} for {RPC_BAN_PERIOD:?}");
                    RPC_BAN_PERIOD
                } else {
                    RPC_FAILURE_KEEP_ALIVE
                };

                self.per_ip_failures_expiry.reset(expiry_key, expiry);
            }
        }
    }
}
