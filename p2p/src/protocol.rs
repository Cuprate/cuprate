pub mod internal_network;

pub use internal_network::{InternalMessageRequest, InternalMessageResponse};

use monero_wire::messages::CoreSyncData;

/// A request to a [`tower::Service`] that handles sync states.
pub enum CoreSyncDataRequest {
    /// Get our [`CoreSyncData`].
    GetOurs,
    /// Handle an incoming [`CoreSyncData`].
    NewIncoming(CoreSyncData),
}

/// A response from a [`tower::Service`] that handles sync states.
pub enum CoreSyncDataResponse {
    /// Our [`CoreSyncData`]
    Ours(CoreSyncData),
    /// The incoming [`CoreSyncData`] is ok.
    Ok,
}

/// The direction of a connection.
pub enum Direction {
    /// An inbound connection.
    Inbound,
    /// An outbound connection.
    Outbound,
}
