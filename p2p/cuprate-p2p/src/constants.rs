use std::time::Duration;

pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) const SEED_CONNECTION_RETRY_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) const CONCURRENT_PEER_LIST_REQUESTS: usize = 3;

pub(crate) const MAX_SEED_CONNECTIONS: usize = 3;

pub(crate) const PEER_FIND_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) const OUTBOUND_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// The duration of a short ban (1 hour).
pub(crate) const SHORT_BAN: Duration = Duration::from_secs(60 * 60);

/// When broadcasting transactions we use the [Poisson](rand_distr::Poisson) distribution to generate the
/// number of seconds we should wait between broadcasts.
///
/// However, if we were to just input the average number of seconds into the [Poisson](rand_distr::Poisson) distribution
/// we would get outputs in 1 second increments which is not granular enough. If we were to use milliseconds we would get
/// outputs with not enough variance.
///
/// So instead we scale the average number of seconds when giving the average to the [Poisson](rand_distr::Poisson) distribution,
/// and then we do the reverse when retrieving values, this value is the scale.
///
/// Idea taken from monero-core: https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/cryptonote_protocol/levin_notify.cpp#L71
pub(crate) const DIFFUSION_POISSON_SECOND_FRACTION: f32 = 4.0;

pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND: f32 = 5.0;

pub(crate) const DIFFUSION_FLUSH_AVERAGE_SECONDS_OUTBOUND: f32 =
    DIFFUSION_FLUSH_AVERAGE_SECONDS_INBOUND / 2.0;

pub(crate) const SOFT_TX_MESSAGE_SIZE_SIZE_LIMIT: usize = 1024 * 1024 * 60;

/// The limit on the amount of transactions kept in the broadcast channel.
///
/// A transaction is kept in the broadcast channel until all nodes have broadcast it.
///
/// Because of internal implementation details this limit will ALWAYS be hit i.e. a tx will stay in the
/// channel until [`MAX_TXS_IN_BROADCAST_CHANNEL`] more txs are added.
pub(crate) const MAX_TXS_IN_BROADCAST_CHANNEL: usize = 250;
