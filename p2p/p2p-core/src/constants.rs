//! Constants used around monero-p2p

use std::time::Duration;

/// The request timeout - the time we give a peer to respond to a request.
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// The timeout used when sending messages to a peer.
///
/// TODO: Make this configurable?
/// TODO: Is this a good default.
pub(crate) const SENDING_TIMEOUT: Duration = Duration::from_secs(20);

/// The interval between timed syncs.
///
/// TODO: Make this configurable?
/// TODO: Is this a good default.
pub(crate) const TIMEOUT_INTERVAL: Duration = Duration::from_secs(61);

/// This is a Cuprate specific constant.
///
/// When completing a handshake monerod might send protocol messages before the handshake is actually
/// complete, this is a problem for Cuprate as we must complete the handshake before responding to any
/// protocol requests. So when we receive a protocol message during a handshake we keep them around to handle
/// after the handshake.
///
/// Because we use the [bytes crate](https://crates.io/crates/bytes) in monero-wire for zero-copy parsing
/// it is not safe to keep too many of these messages around for long.
pub(crate) const MAX_EAGER_PROTOCOL_MESSAGES: usize = 1;

/// A timeout put on pings during handshakes.
///
/// When we receive an inbound connection we open an outbound connection to the node and send a ping message
/// to see if we can reach the node, so we can add it to our address book.
///
/// This timeout must be significantly shorter than [`HANDSHAKE_TIMEOUT`] so we don't drop inbound connections that
/// don't have ports open.
pub(crate) const PING_TIMEOUT: Duration = Duration::from_secs(10);

/// A timeout for a handshake - the handshake must complete before this.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) const MAX_PEERS_IN_PEER_LIST_MESSAGE: usize = 250;
