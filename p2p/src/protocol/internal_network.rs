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
    GetObjectsResponse, GetTxPoolCompliment, HandshakeRequest, HandshakeResponse, NewBlock,
    NewFluffyBlock, NewTransactions, PingResponse, SupportFlagsResponse, TimedSyncRequest,
    TimedSyncResponse,
};

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
