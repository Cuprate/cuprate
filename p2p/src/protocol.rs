pub mod internal_network;

pub use internal_network::{InternalMessageRequest, InternalMessageResponse};

pub struct CoreSyncDataRequest;

use monero_wire::messages::CoreSyncData;
pub struct CoreSyncDataResponse(pub CoreSyncData);

pub enum Direction {
    Inbound,
    Outbound,
}
