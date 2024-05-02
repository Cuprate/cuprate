use std::sync::{Arc, OnceLock};

pub struct SharedError<T>(Arc<OnceLock<T>>);

impl<T> Clone for SharedError<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for SharedError<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SharedError<T> {
    pub fn new() -> Self {
        Self(Arc::new(OnceLock::new()))
    }

    pub fn try_get_err(&self) -> Option<&T> {
        self.0.get()
    }

    pub fn try_insert_err(&self, err: T) -> Result<(), &T> {
        self.0.set(err).map_err(|_| self.0.get().unwrap())
    }
}

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
    BucketError(#[from] monero_wire::BucketError),
    #[error("handshake error: {0}")]
    Handshake(#[from] crate::client::HandshakeError),
    #[error("i/o error: {0}")]
    IO(#[from] std::io::Error),
}
