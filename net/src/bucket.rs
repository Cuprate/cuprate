pub mod header;
pub mod stream;

use thiserror::Error;

use crate::messages::{admin, MessageNotification, MessageRequest, MessageResponse};
use header::{Flags, P2pCommand};

pub use header::BucketHead;
pub use stream::BucketStream;

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
    ParseFailed(&'static str),
    /// More bytes needed to parse data.
    #[error("More bytes needed to parse data")]
    NotEnoughBytes,
    /// Epee serde error.
    #[error("Epee serde error: {0}")]
    EpeeEncodingError(#[from] epee_serde::Error),
    /// IO Error.
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    /// Peer sent an error response code.
    #[error("Peer sent an error response code: {0}")]
    Error(i32),
}

pub const PROTOCOL_VERSION: u32 = 1;
pub const LEVIN_SIGNATURE: u64 = 0x0101010101012101;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BucketBody {
    Request(Box<MessageRequest>),
    Response(Box<MessageResponse>),
    Notification(Box<MessageNotification>),
}

impl BucketBody {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            // Verify unwraps are ok here
            BucketBody::Request(req) => req.to_bytes().unwrap(),
            BucketBody::Response(res) => res.to_bytes().unwrap(),
            BucketBody::Notification(noti) => noti.to_bytes().unwrap(),
        }
    }

    pub fn flags(&self) -> Flags {
        match self {
            BucketBody::Request(_) => Flags::REQUEST,
            BucketBody::Response(_) => Flags::RESPONSE,
            BucketBody::Notification(_) => Flags::REQUEST,
        }
    }

    pub fn command(&self) -> P2pCommand {
        match self {
            BucketBody::Request(req) => req.command(),
            BucketBody::Response(res) => res.command(),
            BucketBody::Notification(noti) => noti.command(),
        }
    }

    pub fn have_to_return_data(&self) -> bool {
        matches!(self, BucketBody::Request(_))
    }

    pub fn build_full_bucket_bytes(&self) -> Vec<u8> {
        let mut body_bytes = self.to_bytes();

        let header = BucketHead {
            signature: LEVIN_SIGNATURE,
            protocol_version: PROTOCOL_VERSION,
            size: body_bytes.len() as u64,
            have_to_return_data: self.have_to_return_data(),
            command: self.command(),
            return_code: 0,
            flags: self.flags(),
        };

        let mut out = header.to_bytes();
        out.append(&mut body_bytes);

        out
    }

    fn from_bytes(
        bytes: &[u8],
        have_to_return_data: bool,
        flags: Flags,
        command: P2pCommand,
    ) -> Result<BucketBody, BucketError> {
        if flags == Flags::REQUEST && have_to_return_data {
            let message_req = match command {
                P2pCommand::Handshake => MessageRequest::Handshake(epee_serde::from_bytes(bytes)?),
                P2pCommand::TimedSync => MessageRequest::TimedSync(epee_serde::from_bytes(bytes)?),
                P2pCommand::Ping => MessageRequest::Ping(admin::PingRequest),
                P2pCommand::SupportFlags => {
                    MessageRequest::SupportFlags(admin::SupportFlagsRequest)
                }
                _ => {
                    return Err(BucketError::ParseFailed(
                        "Message has invalid command and flag combination",
                    ))
                }
            };
            return Ok(BucketBody::Request(Box::new(message_req)));
        }

        if flags == Flags::RESPONSE && !have_to_return_data {
            let message_res = match command {
                P2pCommand::Handshake => MessageResponse::Handshake(epee_serde::from_bytes(bytes)?),
                P2pCommand::TimedSync => MessageResponse::TimedSync(epee_serde::from_bytes(bytes)?),
                P2pCommand::Ping => MessageResponse::Ping(epee_serde::from_bytes(bytes)?),
                P2pCommand::SupportFlags => {
                    MessageResponse::SupportFlags(epee_serde::from_bytes(bytes)?)
                }
                _ => {
                    return Err(BucketError::ParseFailed(
                        "Message has invalid command and flag combination",
                    ))
                }
            };
            return Ok(BucketBody::Response(Box::new(message_res)));
        }

        if flags == Flags::REQUEST && !have_to_return_data {
            let message_notify = match command {
                P2pCommand::NewBlock => {
                    MessageNotification::NewBlock(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::NewTransactions => {
                    MessageNotification::NewTransactions(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::RequestGetObject => {
                    MessageNotification::RequestGetObject(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::ResponseGetObject => {
                    MessageNotification::ResponseGetObject(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::RequestChain => {
                    MessageNotification::RequestChain(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::ResponseChainEntry => {
                    MessageNotification::ResponseChainEntry(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::NewFluffyBlock => {
                    MessageNotification::NewFluffyBlock(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::RequestFluffyMissingTx => {
                    MessageNotification::RequestFluffyMissingTx(epee_serde::from_bytes(bytes)?)
                }
                P2pCommand::GetTxPoolComplement => {
                    MessageNotification::GetTxPoolComplement(epee_serde::from_bytes(bytes)?)
                }
                _ => {
                    return Err(BucketError::ParseFailed(
                        "Message has invalid command and flag combination",
                    ))
                }
            };

            return Ok(BucketBody::Notification(Box::new(message_notify)));
        }

        Err(BucketError::ParseFailed(
            "Message has invalid header combination",
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bucket {
    pub header: BucketHead,
    pub body: BucketBody,
}
