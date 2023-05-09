pub mod client;
pub mod connection;
pub mod connector;
pub mod handshaker;
pub mod load_tracked_client;

#[cfg(test)]
mod tests;

use monero_wire::levin::BucketError;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy)]
pub enum RequestServiceError {}

#[derive(Debug, Error, Clone, Copy)]
pub enum PeerError {
    #[error("The connection task has closed.")]
    ConnectionTaskClosed,
}

pub use client::Client;
pub use client::ConnectionInfo;
pub use connection::Connection;
pub use connector::{Connector, OutboundConnectorRequest};
pub use handshaker::Handshaker;
pub use load_tracked_client::LoadTrackedClient;
