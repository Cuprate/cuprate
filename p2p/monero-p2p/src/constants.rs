//! Constants used around monero-p2p

use monero_wire::ProtocolMessage;
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

/// This is Cuprate specific - monerod will send protocol messages before a handshake is complete in
/// certain circumstances i.e. monerod will send a [`ProtocolMessage::GetTxPoolCompliment`] if our node
/// is its first connection, and we are at the same height.
///
/// Cuprate needs to complete a handshake before any protocol messages can be handled though, so we keep
/// them around to handle when the handshake is done. We don't want this to grow forever though, so we cap
/// the amount we can receive.
pub(crate) const MAX_EAGER_PROTOCOL_MESSAGES: usize = 2;

/// A timeout for a handshake - the handshake must complete before this.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) const MAX_PEERS_IN_PEER_LIST_MESSAGE: usize = 250;
