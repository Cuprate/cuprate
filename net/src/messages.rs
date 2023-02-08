pub mod admin;
pub mod common;
pub mod protocol;

pub use common::{BasicNodeData, CoreSyncData, PeerID, PeerListEntryBase};

use crate::bucket::header::P2pCommand;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageResponse {
    Handshake(admin::HandshakeResponse),
    TimedSync(admin::TimedSyncResponse),
    Ping(admin::PingResponse),
    SupportFlags(admin::SupportFlagsResponse),
}

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
