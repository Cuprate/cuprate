#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("error with peer response: {0}")]
    ResponseError(&'static str),
    #[error("the peer sent an incorrect response to our request")]
    PeerSentIncorrectResponse,
    #[error("bucket error")]
    BucketError(#[from] monero_wire::BucketError),
    #[error("handshake error: {0}")]
    Handshake(#[from] crate::client::HandshakeError),
    #[error("i/o error: {0}")]
    IO(#[from] std::io::Error),
}
