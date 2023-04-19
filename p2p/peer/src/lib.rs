pub mod connection;
pub mod handshaker;
pub mod client;

use thiserror::Error;
use monero_wire::levin::BucketError;

#[derive(Debug, Error, Clone, Copy)]
pub enum RequestServiceError {}

#[derive(Debug, Error, Clone, Copy)]
pub enum PeerError {
    #[error("Peer is on a different network")]
    PeerIsOnAnotherNetwork,
    #[error("Peer sent an unexpected response")]
    PeerSentUnSolicitedResponse,
    #[error("Internal service did not respond when required")]
    InternalServiceDidNotRespond,
    #[error("Connection to peer has been terminated")]
    PeerConnectionClosed,
    #[error("The Client `internal` channel was closed")]
    ClientChannelClosed,
    #[error("The Peer sent an unexpected response")]
    PeerSentUnexpectedResponse,
    #[error("The peer sent a bad response: {0}")]
    ResponseError(&'static str),
    #[error("Internal service error: {0}")]
    InternalService(#[from] RequestServiceError),
    #[error("Internal peer sync channel closed")]
    InternalPeerSyncChannelClosed,
    #[error("Levin Error")]
    LevinError, // remove me, this is just temporary
}

impl From<BucketError> for PeerError {
    fn from(_: BucketError) -> Self {
        PeerError::LevinError
    }
}
