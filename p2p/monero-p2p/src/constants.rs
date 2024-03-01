//! Constants used around monero-p2p

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
pub(crate) const DEFAULT_RTT: Duration = Duration::from_secs(REQUEST_TIMEOUT.as_secs() + 1);

/// The decay_ns used in the peer load measurement.
pub(crate) const PEAK_EWMA_DECAY_NS: f64 = Duration::from_secs(300).as_nanos() as f64;
