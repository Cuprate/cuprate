mod connection;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeerError {
    #[error("Peer sent an unexpected response")]
    PeerSentUnSolicitedResponse, 
    #[error("Internal service did not respond when required")]
    InternalServiceDidNotRespond,
}
