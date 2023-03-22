pub mod client;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy)]
pub enum PeerError {
    #[error("Peer sent an unexpected response")]
    PeerSentUnSolicitedResponse,
    #[error("Internal service did not respond when required")]
    InternalServiceDidNotRespond,
    #[error("Connection to peer has been terminated")]
    PeerConnectionClosed,
    #[error("The Client `internal` channel was closed")]
    ClientChannelClosed,
    #[error("Levin Error")]
    LevinError, // remove me, this is just temporary
}
