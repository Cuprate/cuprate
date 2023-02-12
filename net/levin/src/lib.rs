pub mod bucket_stream;
pub mod header;
pub mod protocol_machine;

pub use bucket_stream::BucketStream;
pub use header::BucketHead;

use thiserror::Error;

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
    /// Failed to parse data.
    #[error("Failed to parse data: {0}")]
    ParseFailed(String),
    /// More bytes needed to parse data.
    #[error("More bytes needed to parse data")]
    NotEnoughBytes,
    /// IO Error.
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    /// Peer sent an error response code.
    #[error("Peer sent an error response code: {0}")]
    Error(i32),
}

pub const PROTOCOL_VERSION: u32 = 1;
pub const LEVIN_SIGNATURE: u64 = 0x0101010101012101;

pub struct Bucket {
    header: BucketHead,
    body: Vec<u8>,
}

impl Bucket {
    fn to_bytes(&self) -> Vec<u8> {
        [self.header.to_bytes(), self.body.clone()].concat()
    }
}

pub trait BodyDeEnc: Sized {
    fn decode_body(buf: &[u8], command: u32) -> Result<Self, BucketError>;

    /// returns the encoded body and the "command"
    fn encode_body(&self) -> (Vec<u8>, u32);
}

#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound
}

#[derive(Debug)]
pub enum ISMOutput<A, R, N, Q, S> {
    Connect(A),
    Disconnect(A, R),
    WriteNotification(A, N),
    WriteRequest(A, Q),
    WriteResponse(A, S),
    SetTimer(std::time::Duration),
}

pub trait InternalStateMachine:
    Iterator<
    Item = ISMOutput<
        Self::PeerID,
        Self::DisconnectReason,
        Self::BodyNotification,
        Self::BodyRequest,
        Self::BodyResponse,
    >,
>
{
    type PeerID: std::hash::Hash + std::cmp::Eq + Clone;

    type BodyResponse: BodyDeEnc;
    type BodyRequest: BodyDeEnc;
    type BodyNotification: BodyDeEnc;

    type DisconnectReason;

    fn tick(&mut self);

    fn wake(&mut self);

    fn connected(&mut self, addr: &Self::PeerID, direction: Direction);
    fn disconnected(&mut self, addr: &Self::PeerID);

    fn received_response(&mut self, addr: &Self::PeerID, body: Self::BodyResponse);
    fn received_request(&mut self, addr: &Self::PeerID, body: Self::BodyRequest);
    fn received_notification(&mut self, addr: &Self::PeerID, body: Self::BodyNotification);

    fn error_decoding_bucket(&mut self, error: BucketError);
}
