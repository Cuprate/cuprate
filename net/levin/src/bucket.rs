pub mod bucket_sink;
pub mod bucket_stream;
pub mod header;

use bytes::Bytes;
pub use header::BucketHead;

use thiserror::Error;

/// Possible Errors when working with levin buckets
#[derive(Error, Debug)]
pub enum BucketError {
    /// Unsupported p2p command.
    #[error("Unsupported p2p command: {0}")]
    UnsupportedP2pCommand(u32),
    /// Revived header with incorrect signature.
    #[error("Revived header with incorrect signature: {0}")]
    IncorrectSignature(u64),
    /// Header contains unknown flags.
    #[error("Header contains unknown flags")]
    UnknownFlags,
    /// Revived header with unknown protocol version.
    #[error("Revived header with unknown protocol version: {0}")]
    UnknownProtocolVersion(u32),
    /// Failed to decode bucket body.
    #[error("Failed to decode bucket body: {0}")]
    FailedToDecodeBucketBody(String),
    /// Failed to encode bucket body.
    #[error("Failed to encode bucket body: {0}")]
    FailedToEncodeBucketBody(String),
    /// IO Error.
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
}

const PROTOCOL_VERSION: u32 = 1;
const LEVIN_SIGNATURE: u64 = 0x0101010101012101;

/// A levin Bucket
#[derive(Debug)]
pub struct Bucket {
    pub header: BucketHead,
    pub body: Bytes,
}

impl Bucket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = self.header.to_bytes();
        buf.extend(self.body.iter());
        buf.into()
    }
}

/// An enum representing if the message is a request or response
#[derive(Debug)]
pub enum MessageType {
    /// Request
    Request,
    /// Response
    Response,
}

impl From<MessageType> for header::Flags {
    fn from(val: MessageType) -> Self {
        match val {
            MessageType::Request => header::REQUEST,
            MessageType::Response => header::RESPONSE,
        }
    }
}

impl TryInto<MessageType> for header::Flags {
    type Error = BucketError;
    fn try_into(self) -> Result<MessageType, Self::Error> {
        if self.is_request() {
            Ok(MessageType::Request)
        } else if self.is_response() {
            Ok(MessageType::Response)
        } else {
            Err(BucketError::UnknownFlags)
        }
    }
}
