pub mod internal_network;
pub mod temp_database;

pub use internal_network::{InternalMessageRequest, InternalMessageResponse};

pub const BLOCKS_IDS_SYNCHRONIZING_DEFAULT_COUNT: usize = 10000;
pub const BLOCKS_IDS_SYNCHRONIZING_MAX_COUNT: usize = 25000;
pub const P2P_MAX_PEERS_IN_HANDSHAKE: usize = 250;

pub enum Direction {
    Inbound,
    Outbound,
}