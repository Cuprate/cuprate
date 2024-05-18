use std::time::Duration;

/// The timeout we set on handshakes.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// The maximum amount of connections to make to seed nodes for when we need peers.
pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

/// The timeout for when we fail to find a peer to connect to.
pub(crate) const OUTBOUND_CONNECTION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);

/// The default amount of time between inbound diffusion flushes.
pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND: Duration = Duration::from_secs(5);

/// The default amount of time between outbound diffusion flushes.
///
/// Shorter than inbound as we control these connections.
pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND: Duration = Duration::from_millis(2500);

/// This size limit on [`NewTransactions`](monero_wire::protocol::NewTransactions) messages that we create.
pub(crate) const SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT: usize = 10 * 1024 * 1024;

/// The amount of transactions in the broadcast queue. When this value is hit old transactions will be dropped from
/// the queue.
///
/// Because of internal implementation details this value is _always_ hit, i.e. transactions will not be dropped until
/// 50 more after it are added.
pub(crate) const MAX_TXS_IN_BROADCAST_CHANNEL: usize = 50;

/// The durations of a short ban.
pub(crate) const SHORT_BAN: Duration = Duration::from_secs(60 * 10);
