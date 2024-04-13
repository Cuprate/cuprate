//! Constants used around monero-p2p

use monero_wire::ProtocolMessage;
use std::time::Duration;

/// The request timeout - the time we give a peer to respond to a request.
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Default round trip time - the default time we say a peer should take to respond to a request.
///
/// This is used to initialise the peer load measurement, the idea behind setting a high value for this
/// is from `zebra` - it prevents new connections from being selected over peers we have sent requests
/// to already.
///
// TODO: Is this a good idea?
pub(crate) const DEFAULT_RTT: Duration = Duration::from_secs(10);

/// The decay_ns used in the peer load measurement.
pub(crate) const PEAK_EWMA_DECAY_NS: f64 = Duration::from_secs(300).as_nanos() as f64;

/// This is Cuprate specific - monerod will send protocol messages before a handshake is complete in
/// certain circumstances i.e. monerod will send a [`ProtocolMessage::GetTxPoolCompliment`] if our node
/// is its first connection, and we are at the same height.
///
/// Cuprate needs to complete a handshake before any protocol messages can be handled though, so we keep
/// them around to handle when the handshake is done. We don't want this to grow forever though, so we cap
/// the amount we can receive.
pub(crate) const MAX_EAGER_PROTOCOL_MESSAGES: usize = 2;

/// A timeout for a handshake - the handshake must complete before this.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) const MAX_PEERS_IN_PEER_LIST_MESSAGE: usize = 250;
