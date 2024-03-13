use std::time::Duration;

pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) const SEED_CONNECTION_RETRY_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) const CONCURRENT_PEER_LIST_REQUESTS: usize = 3;

pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

pub(crate) const PEER_FIND_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const OUTBOUND_CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);

/// The duration of a short ban (1 hour).
pub(crate) const SHORT_BAN: Duration = Duration::from_secs(60 * 60);
