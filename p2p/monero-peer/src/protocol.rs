/// This module defines InternalRequests and InternalResponses. Cuprate's P2P works by translating network messages into an internal
/// request/ response, this is easy for levin "requests" and "responses" (admin messages) but takes a bit more work with "notifications"
/// (protocol messages).
///
/// Some notifications are easy to translate, like `GetObjectsRequest` is obviously a request but others like `NewFluffyBlock` are a
/// bit tricker. To translate a `NewFluffyBlock` into a request/ response we will have to look to see if we asked for `FluffyMissingTransactionsRequest`
/// if we have we interpret `NewFluffyBlock` as a response if not its a request that doesn't require a response.
///
/// Here is every P2P request/ response. *note admin messages are already request/ response so "Handshake" is actually made of a HandshakeRequest & HandshakeResponse
///
/// Admin:
///     Handshake,
///     TimedSync,
///     Ping,
///     SupportFlags
/// Protocol:
///     Request: GetObjectsRequest,                 Response: GetObjectsResponse,
///     Request: ChainRequest,                      Response: ChainResponse,
///     Request: FluffyMissingTransactionsRequest,  Response: NewFluffyBlock,  <- these 2 could be requests or responses
///     Request: GetTxPoolCompliment,               Response: NewTransactions, <-
///     Request: NewBlock,                          Response: None,
///     Request: NewFluffyBlock,                    Response: None,
///     Request: NewTransactions,                   Response: None
///
///
use monero_wire::{
    admin::{
        HandshakeRequest, HandshakeResponse, PingResponse, SupportFlagsResponse, TimedSyncRequest,
        TimedSyncResponse,
    },
    protocol::{
        ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
        GetObjectsResponse, GetTxPoolCompliment, NewBlock, NewFluffyBlock, NewTransactions,
    }, 
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

pub enum PeerRequest {
    Handshake(HandshakeRequest),
    TimedSync(TimedSyncRequest),
    Ping,
    SupportFlags,

    GetObjects(GetObjectsRequest),
    GetChain(ChainRequest),
    FluffyMissingTxs(FluffyMissingTransactionsRequest),
    GetTxPoolCompliment(GetTxPoolCompliment),
    NewBlock(NewBlock),
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
}

impl PeerRequest {
    pub fn id(&self) -> MessageID {
        match self {
            PeerRequest::Handshake(_) => MessageID::Handshake,
            PeerRequest::TimedSync(_) => MessageID::TimedSync,
            PeerRequest::Ping => MessageID::Ping,
            PeerRequest::SupportFlags => MessageID::SupportFlags,

            PeerRequest::GetObjects(_) => MessageID::GetObjects,
            PeerRequest::GetChain(_) => MessageID::GetChain,
            PeerRequest::FluffyMissingTxs(_) => MessageID::FluffyMissingTxs,
            PeerRequest::GetTxPoolCompliment(_) => MessageID::GetTxPoolCompliment,
            PeerRequest::NewBlock(_) => MessageID::NewBlock,
            PeerRequest::NewFluffyBlock(_) => MessageID::NewFluffyBlock,
            PeerRequest::NewTransactions(_) => MessageID::NewTransactions,
        }
    }

    pub fn needs_response(&self) -> bool {
        match self {
            PeerRequest::NewBlock(_)
            | PeerRequest::NewFluffyBlock(_)
            | PeerRequest::NewTransactions(_) => false,
            _ => true,
        }
    }
}

pub enum PeerResponse {
    Handshake(HandshakeResponse),
    TimedSync(TimedSyncResponse),
    Ping(PingResponse),
    SupportFlags(SupportFlagsResponse),

    GetObjects(GetObjectsResponse),
    GetChain(ChainResponse),
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
    NA,
}

impl PeerResponse {
    pub fn id(&self) -> MessageID {
        match self {
            PeerResponse::Handshake(_) => MessageID::Handshake,
            PeerResponse::TimedSync(_) => MessageID::TimedSync,
            PeerResponse::Ping(_) => MessageID::Ping,
            PeerResponse::SupportFlags(_) => MessageID::SupportFlags,

            PeerResponse::GetObjects(_) => MessageID::GetObjects,
            PeerResponse::GetChain(_) => MessageID::GetChain,
            PeerResponse::NewFluffyBlock(_) => MessageID::NewBlock,
            PeerResponse::NewTransactions(_) => MessageID::NewFluffyBlock,

            PeerResponse::NA => panic!("Can't get message ID for a non existent response"),
        }
    }
}
