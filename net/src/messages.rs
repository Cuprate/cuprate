pub mod admin;
pub mod common;
pub mod protocol;

pub use common::{BasicNodeData, CoreSyncData, PeerID, PeerListEntryBase};

use crate::P2pCommand;
use levin::BodyDeEnc;

fn zero_val<T: From<u8>>() -> T {
    T::from(0_u8)
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

macro_rules! to_bytes {
    ($($variant:ident), +)=> {
        pub fn to_bytes(&self) -> Result<Vec<u8>, epee_serde::Error> {
            match self {
                $(Self::$variant(temp) => epee_serde::to_bytes(temp),) +
            }
        }
    };
}

macro_rules! command {
    ($($variant:ident), +) => {
        pub fn command(&self) -> P2pCommand {
            match self {
                $(Self::$variant(_) => P2pCommand::$variant,) +
            }
        }
    };
}

macro_rules! decode_body {
    ($($variant:ident), +) => {
        fn decode_body(buf: &[u8], command: u32) -> Result<Self, levin::BucketError> {
            match P2pCommand::try_from(command)? {
                $(P2pCommand::$variant => Ok(Self::$variant(
                    match epee_serde::from_bytes(buf) {
                        Ok(body) => body,
                        Err(e) => return Err(levin::BucketError::ParseFailed(format!("Failed to decode: {}, err: {}", stringify!($variant), e))),
                    }
                )),) +

                _ => Err(levin::BucketError::ParseFailed("Invalid header(command, have_to_return_data) combination".to_string()))
            }
        }

    };
}

macro_rules! impl_body_dec_enc {
    ($enum:ident, $($variant:ident), +) => {
        impl BodyDeEnc for $enum {
            fn encode_body(&self) -> (Vec<u8>, u32) {
                (self.to_bytes().unwrap(), self.command() as u32)
            }

            decode_body!($($variant), +);

        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRequest {
    Handshake(admin::HandshakeRequest),
    TimedSync(admin::TimedSyncRequest),
    Ping(admin::PingRequest),
    SupportFlags(admin::SupportFlagsRequest),
}

impl MessageRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, epee_serde::Error> {
        match self {
            MessageRequest::Handshake(temp) => epee_serde::to_bytes(temp),
            MessageRequest::TimedSync(temp) => epee_serde::to_bytes(temp),
            MessageRequest::Ping(_) => Ok(vec![]),
            MessageRequest::SupportFlags(_) => Ok(vec![]),
        }
    }

    command!(Handshake, TimedSync, Ping, SupportFlags);
}

impl BodyDeEnc for MessageRequest {
    fn encode_body(&self) -> (Vec<u8>, u32) {
        (self.to_bytes().unwrap(), self.command() as u32)
    }

    fn decode_body(buf: &[u8], command: u32) -> Result<Self, levin::BucketError> {
        match P2pCommand::try_from(command)? {
            P2pCommand::Handshake => Ok(Self::Handshake(match epee_serde::from_bytes(buf) {
                Ok(body) => body,
                Err(e) => {
                    return Err(levin::BucketError::ParseFailed(format!(
                        "Failed to decode: Handshake, err: {}",
                        e
                    )))
                }
            })),
            P2pCommand::TimedSync => Ok(Self::TimedSync(match epee_serde::from_bytes(buf) {
                Ok(body) => body,
                Err(e) => {
                    return Err(levin::BucketError::ParseFailed(format!(
                        "Failed to decode: Handshake, err: {}",
                        e
                    )))
                }
            })),

            P2pCommand::Ping => Ok(MessageRequest::Ping(admin::PingRequest)),
            P2pCommand::SupportFlags => {
                Ok(MessageRequest::SupportFlags(admin::SupportFlagsRequest))
            }

            _ => Err(levin::BucketError::ParseFailed(
                "Invalid header(command, have_to_return_data) combination".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageResponse {
    Handshake(admin::HandshakeResponse),
    TimedSync(admin::TimedSyncResponse),
    Ping(admin::PingResponse),
    SupportFlags(admin::SupportFlagsResponse),
}

impl_body_dec_enc!(MessageResponse, Handshake, TimedSync, Ping, SupportFlags);

impl MessageResponse {
    to_bytes!(Handshake, TimedSync, Ping, SupportFlags);
    command!(Handshake, TimedSync, Ping, SupportFlags);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageNotification {
    NewBlock(protocol::NewBlock),
    NewTransactions(protocol::NewTransactions),
    RequestGetObject(protocol::GetObjectsRequest),
    ResponseGetObject(protocol::GetObjectsResponse),
    RequestChain(protocol::ChainRequest),
    ResponseChainEntry(protocol::ChainResponse),
    NewFluffyBlock(protocol::NewFluffyBlock),
    RequestFluffyMissingTx(protocol::FluffyMissingTransactionsRequest),
    GetTxPoolComplement(protocol::TxPoolCompliment),
}

impl_body_dec_enc!(
    MessageNotification,
    NewBlock,
    NewTransactions,
    RequestGetObject,
    ResponseGetObject,
    RequestChain,
    ResponseChainEntry,
    NewFluffyBlock,
    RequestFluffyMissingTx,
    GetTxPoolComplement
);

impl MessageNotification {
    to_bytes!(
        NewBlock,
        NewTransactions,
        RequestGetObject,
        ResponseGetObject,
        RequestChain,
        ResponseChainEntry,
        NewFluffyBlock,
        RequestFluffyMissingTx,
        GetTxPoolComplement
    );
    command!(
        NewBlock,
        NewTransactions,
        RequestGetObject,
        ResponseGetObject,
        RequestChain,
        ResponseChainEntry,
        NewFluffyBlock,
        RequestFluffyMissingTx,
        GetTxPoolComplement
    );
}
