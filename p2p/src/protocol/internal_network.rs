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
use monero_wire::{
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
    GetObjectsResponse, GetTxPoolCompliment, HandshakeRequest, HandshakeResponse, Message,
    NewBlock, NewFluffyBlock, NewTransactions, PingResponse, SupportFlagsResponse,
    TimedSyncRequest, TimedSyncResponse,
};

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
    GetTxPollCompliment,
    NewBlock,
    NewFluffyBlock,
    NewTransactions,
}

pub enum Request {
    Handshake(HandshakeRequest),
    TimedSync(TimedSyncRequest),
    Ping,
    SupportFlags,

    GetObjects(GetObjectsRequest),
    GetChain(ChainRequest),
    FluffyMissingTxs(FluffyMissingTransactionsRequest),
    GetTxPollCompliment(GetTxPoolCompliment),
    NewBlock(NewBlock),
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
}

impl Request {
    pub fn id(&self) -> MessageID {
        match self {
            Request::Handshake(_) => MessageID::Handshake,
            Request::TimedSync(_) => MessageID::TimedSync,
            Request::Ping => MessageID::Ping,
            Request::SupportFlags => MessageID::SupportFlags,

            Request::GetObjects(_) => MessageID::GetObjects,
            Request::GetChain(_) => MessageID::GetChain,
            Request::FluffyMissingTxs(_) => MessageID::FluffyMissingTxs,
            Request::GetTxPollCompliment(_) => MessageID::GetTxPollCompliment,
            Request::NewBlock(_) => MessageID::NewBlock,
            Request::NewFluffyBlock(_) => MessageID::NewFluffyBlock,
            Request::NewTransactions(_) => MessageID::NewTransactions,
        }
    }

    pub fn needs_response(&self) -> bool {
        match self {
            Request::NewBlock(_) | Request::NewFluffyBlock(_) | Request::NewTransactions(_) => {
                false
            }
            _ => true,
        }
    }
}

pub enum Response {
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

impl Response {
    pub fn id(&self) -> MessageID {
        match self {
            Response::Handshake(_) => MessageID::Handshake,
            Response::TimedSync(_) => MessageID::TimedSync,
            Response::Ping(_) => MessageID::Ping,
            Response::SupportFlags(_) => MessageID::SupportFlags,

            Response::GetObjects(_) => MessageID::GetObjects,
            Response::GetChain(_) => MessageID::GetChain,
            Response::NewFluffyBlock(_) => MessageID::NewBlock,
            Response::NewTransactions(_) => MessageID::NewFluffyBlock,

            Response::NA => panic!("Can't get message ID for a non existent response"),
        }
    }
}
