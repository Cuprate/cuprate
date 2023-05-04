use core::time::Duration;

use monero_wire::messages::common::PeerSupportFlags;

pub const CUPRATE_SUPPORT_FLAGS: PeerSupportFlags =
    PeerSupportFlags::get_support_flag_fluffy_blocks();

pub const DEFAULT_TARGET_OUT_PEERS: usize = 20;

pub const DEFAULT_LOAD_OUT_PEERS_MULTIPLIER: usize = 3;

pub const DEFAULT_IN_PEERS: usize = 20;

pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// The maximum size of the address books white list.
/// This number is copied from monerod.
pub const MAX_WHITE_LIST_PEERS: usize = 1000;

/// The maximum size of the address books gray list.
/// This number is copied from monerod.
pub const MAX_GRAY_LIST_PEERS: usize = 5000;

/// The max amount of peers that can be sent in one
/// message.
pub const P2P_MAX_PEERS_IN_HANDSHAKE: usize = 250;

/// The timeout for sending a message to a remote peer,
/// and receiving a response from a remote peer.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// The default RTT estimate for peer responses.
///
/// We choose a high value for the default RTT, so that new peers must prove they
/// are fast, before we prefer them to other peers. This is particularly
/// important on testnet, which has a small number of peers, which are often
/// slow.
///
/// Make the default RTT slightly higher than the request timeout.
pub const EWMA_DEFAULT_RTT: Duration = Duration::from_secs(REQUEST_TIMEOUT.as_secs() + 1);

/// The decay time for the EWMA response time metric used for load balancing.
///
/// This should be much larger than the `SYNC_RESTART_TIMEOUT`, so we choose
/// better peers when we restart the sync.
pub const EWMA_DECAY_TIME_NANOS: f64 = 200.0 * NANOS_PER_SECOND;

/// The number of nanoseconds in one second.
const NANOS_PER_SECOND: f64 = 1_000_000_000.0;
