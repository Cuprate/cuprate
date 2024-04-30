use std::time::Duration;

pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const CHAIN_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) const BLOCK_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) const BLOCK_REQUEST_TIMEOUT_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) const SEED_CONNECTION_RETRY_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) const CONCURRENT_PEER_LIST_REQUESTS: usize = 3;

pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

pub(crate) const PEER_FIND_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const OUTBOUND_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// The duration of a short ban (1 hour).
pub(crate) const SHORT_BAN: Duration = Duration::from_secs(60 * 60);

/// The duration of a medium ban (24 hours).
pub(crate) const MEDIUM_BAN: Duration = Duration::from_secs(60 * 60 * 24);

pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND: Duration = Duration::from_secs(5);

pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND: Duration = Duration::from_millis(2500);

pub(crate) const SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT: usize = 1024 * 1024 * 60;

/// The limit on the amount of transactions kept in the broadcast channel.
///
/// A transaction is kept in the broadcast channel until all nodes have broadcast it.
///
/// Because of internal implementation details this limit will ALWAYS be hit i.e. a tx will stay in the
/// channel until [`MAX_TXS_IN_BROADCAST_CHANNEL`] more txs are added.
pub(crate) const MAX_TXS_IN_BROADCAST_CHANNEL: usize = 50;

pub(crate) const INCOMING_BLOCKS_CACHE_SIZE: usize = 10 * 1024 * 1024;

pub(crate) const NUMBER_OF_BLOCKS_TO_REQUEST: usize = 100;

pub(crate) const CHAIN_REQUESTS_TO_SEND: usize = 2;
