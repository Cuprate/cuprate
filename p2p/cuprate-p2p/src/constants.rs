use std::time::Duration;

/// The timeout we set on handshakes.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// The maximum amount of connections to make to seed nodes for when we need peers.
pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

/// The timeout for when we fail to find a peer to connect to.
pub(crate) const OUTBOUND_CONNECTION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND: Duration = Duration::from_secs(5);

pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND: Duration = Duration::from_millis(2500);

pub(crate) const SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT: usize = 10_000_000;

pub(crate) const MAX_TXS_IN_BROADCAST_CHANNEL: usize = 50;
