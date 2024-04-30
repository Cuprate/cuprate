use std::time::Duration;

pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

pub(crate) const PEER_FIND_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const OUTBOUND_CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
