//! This module defines [`PeerRequest`] and [`PeerResponse`]. Cuprate's P2P crates works by translating network messages into an internal
//! request/response enums, this is easy for levin "requests" and "responses" (admin messages) but takes a bit more work with "notifications"
//! (protocol messages).
//!
//! Some notifications are easy to translate, like [`GetObjectsRequest`] is obviously a request but others like [`NewFluffyBlock`] are a
//! bit tricker. To translate a [`NewFluffyBlock`] into a request/ response we will have to look to see if we asked for [`FluffyMissingTransactionsRequest`],
//! if we have, we interpret [`NewFluffyBlock`] as a response, if not, it's a request that doesn't require a response.
//!
//! Here is every P2P request/response.
//!
//! *note admin messages are already request/response so "Handshake" is actually made of a `HandshakeRequest` & `HandshakeResponse`
//!
//! ```md
//! Admin:
//!     Handshake,
//!     TimedSync,
//!     Ping,
//!     SupportFlags
//! Protocol:
//!     Request: GetObjectsRequest,                 Response: GetObjectsResponse,
//!     Request: ChainRequest,                      Response: ChainResponse,
//!     Request: FluffyMissingTransactionsRequest,  Response: NewFluffyBlock,  <- these 2 could be requests or responses
//!     Request: GetTxPoolCompliment,               Response: NewTransactions, <-
//!     Request: NewBlock,                          Response: None,
//!     Request: NewFluffyBlock,                    Response: None,
//!     Request: NewTransactions,                   Response: None
//!```
//!
use cuprate_wire::{
    protocol::{
        ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
        GetObjectsResponse, GetTxPoolCompliment, NewBlock, NewFluffyBlock, NewTransactions,
    },
    AdminRequestMessage, AdminResponseMessage,
};

mod try_from;

/// An enum representing a request/ response combination, so a handshake request
/// and response would have the same [`MessageID`]. This allows associating the
/// correct response to a request.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum MessageID {
    Handshake,
    TimedSync,
    Ping,
    SupportFlags,

    GetObjects,
    GetChain,
    FluffyMissingTxs,
    GetTxPoolCompliment,
    NewBlock,
    NewFluffyBlock,
    NewTransactions,
}

pub enum BroadcastMessage {
    NewFluffyBlock(NewFluffyBlock),
    NewTransaction(NewTransactions),
}

#[derive(Debug, Clone)]
pub enum ProtocolRequest {
    GetObjects(GetObjectsRequest),
    GetChain(ChainRequest),
    FluffyMissingTxs(FluffyMissingTransactionsRequest),
    GetTxPoolCompliment(GetTxPoolCompliment),
    NewBlock(NewBlock),
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
}

#[derive(Debug, Clone)]
pub enum PeerRequest {
    Admin(AdminRequestMessage),
    Protocol(ProtocolRequest),
}

impl PeerRequest {
    pub const fn id(&self) -> MessageID {
        match self {
            Self::Admin(admin_req) => match admin_req {
                AdminRequestMessage::Handshake(_) => MessageID::Handshake,
                AdminRequestMessage::TimedSync(_) => MessageID::TimedSync,
                AdminRequestMessage::Ping => MessageID::Ping,
                AdminRequestMessage::SupportFlags => MessageID::SupportFlags,
            },
            Self::Protocol(protocol_request) => match protocol_request {
                ProtocolRequest::GetObjects(_) => MessageID::GetObjects,
                ProtocolRequest::GetChain(_) => MessageID::GetChain,
                ProtocolRequest::FluffyMissingTxs(_) => MessageID::FluffyMissingTxs,
                ProtocolRequest::GetTxPoolCompliment(_) => MessageID::GetTxPoolCompliment,
                ProtocolRequest::NewBlock(_) => MessageID::NewBlock,
                ProtocolRequest::NewFluffyBlock(_) => MessageID::NewFluffyBlock,
                ProtocolRequest::NewTransactions(_) => MessageID::NewTransactions,
            },
        }
    }

    pub const fn needs_response(&self) -> bool {
        !matches!(
            self,
            Self::Protocol(
                ProtocolRequest::NewBlock(_)
                    | ProtocolRequest::NewFluffyBlock(_)
                    | ProtocolRequest::NewTransactions(_)
            )
        )
    }
}

#[derive(Debug, Clone)]
pub enum ProtocolResponse {
    GetObjects(GetObjectsResponse),
    GetChain(ChainResponse),
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
    FluffyMissingTxs(FluffyMissingTransactionsRequest),
    NA,
}

#[derive(Debug, Clone)]
pub enum PeerResponse {
    Admin(AdminResponseMessage),
    Protocol(ProtocolResponse),
}

impl PeerResponse {
    pub const fn id(&self) -> Option<MessageID> {
        Some(match self {
            Self::Admin(admin_res) => match admin_res {
                AdminResponseMessage::Handshake(_) => MessageID::Handshake,
                AdminResponseMessage::TimedSync(_) => MessageID::TimedSync,
                AdminResponseMessage::Ping(_) => MessageID::Ping,
                AdminResponseMessage::SupportFlags(_) => MessageID::SupportFlags,
            },
            Self::Protocol(protocol_res) => match protocol_res {
                ProtocolResponse::GetObjects(_) => MessageID::GetObjects,
                ProtocolResponse::GetChain(_) => MessageID::GetChain,
                ProtocolResponse::NewFluffyBlock(_) => MessageID::NewBlock,
                ProtocolResponse::NewTransactions(_) => MessageID::NewFluffyBlock,
                ProtocolResponse::FluffyMissingTxs(_) => MessageID::FluffyMissingTxs,

                ProtocolResponse::NA => return None,
            },
        })
    }
}
