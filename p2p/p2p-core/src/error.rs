#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("The connection timed out.")]
    TimedOut,
    #[error("The connection was closed.")]
    ConnectionClosed,
    #[error("The connection tasks client channel was closed")]
    ClientChannelClosed,
    #[error("error with peer response: {0}")]
    ResponseError(&'static str),
    #[error("the peer sent an incorrect response to our request")]
    PeerSentIncorrectResponse,
    #[error("the peer sent an invalid message")]
    PeerSentInvalidMessage,
    #[error("inner service error: {0}")]
    ServiceError(#[from] tower::BoxError),
    #[error("bucket error: {0}")]
    BucketError(#[from] cuprate_wire::BucketError),
    #[error("handshake error: {0}")]
    Handshake(#[from] crate::client::HandshakeError),
    #[error("i/o error: {0}")]
    IO(#[from] std::io::Error),
}
