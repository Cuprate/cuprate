//! This module 

use monero_wire::{messages::{
    admin::{HandshakeRequest, TimedSyncRequest, PingRequest, SupportFlagsRequest},
    admin::{HandshakeResponse, TimedSyncResponse, PingResponse, SupportFlagsResponse},
    protocol::{GetObjectsRequest, ChainRequest, FluffyMissingTransactionsRequest, TxPoolCompliment},
    protocol::{GetObjectsResponse, ChainResponse},
    protocol::{NewFluffyBlock, NewTransactions, NewBlock},
    MessageRequest, MessageResponse, MessageNotification, Message
}, P2pCommand};

use crate::peer::PeerError;

/// A network request - a few requests don't require responses
#[derive(Debug, Clone)]
pub enum Request {
    Handshake(HandshakeRequest),
    TimedSync(TimedSyncRequest),
    Ping(PingRequest),
    SupportFlags(SupportFlagsRequest),
    GetObjects(GetObjectsRequest),
    Chain(ChainRequest),
    FluffyMissingTx(FluffyMissingTransactionsRequest),
    TxPoolCompliment(TxPoolCompliment),
    // these below don't need responses
    NewBlock(NewBlock),
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
}

impl Request {
    pub fn need_response(&self) -> bool {
        !matches!(self, Request::NewBlock(_) | Request::NewFluffyBlock(_) | Request::NewTransactions(_))
    }
    pub fn expected_resp(&self) -> P2pCommand {
        match self {
            Request::Handshake(_) => P2pCommand::Handshake,
            Request::TimedSync(_) => P2pCommand::TimedSync,
            Request::Ping(_) => P2pCommand::Ping,
            Request::SupportFlags(_) => P2pCommand::SupportFlags,
            Request::GetObjects(_ ) => P2pCommand::ResponseGetObject,
            Request::Chain(_) => P2pCommand::ResponseChainEntry,
            Request::FluffyMissingTx(_) => P2pCommand::NewFluffyBlock,
            Request::TxPoolCompliment(_) => P2pCommand::NewTransactions,
            _ => unreachable!("We check if requests need responses before this is called")
        }
    }
}

impl Into<Message> for Request {
    fn into(self) -> Message {
        match self {
            Request::Handshake(hand) => MessageRequest::Handshake(hand).into(),
            Request::TimedSync(timed) => MessageRequest::TimedSync(timed).into(),
            Request::Ping(ping) => MessageRequest::Ping(ping).into(),
            Request::SupportFlags(support) => MessageRequest::SupportFlags(support).into(),

            Request::GetObjects(obj) => MessageNotification::RequestGetObject(obj).into(),
            Request::Chain(chain) => MessageNotification::RequestChain(chain).into(),
            Request::FluffyMissingTx(txs) => MessageNotification::RequestFluffyMissingTx(txs).into(),
            Request::TxPoolCompliment(comp) => MessageNotification::GetTxPoolComplement(comp).into(),
            Request::NewBlock(block) => MessageNotification::NewBlock(block).into(),
            Request::NewFluffyBlock(block) => MessageNotification::NewFluffyBlock(block).into(),
            Request::NewTransactions(txs) => MessageNotification::NewTransactions(txs).into(),
        }
    }
}

impl From<MessageRequest> for Request {
    fn from(value: MessageRequest) -> Self {
        match value {
            MessageRequest::Handshake(hand) => Request::Handshake(hand),
            MessageRequest::TimedSync(timed) => Request::TimedSync(timed),
            MessageRequest::Ping(ping) => Request::Ping(ping),
            MessageRequest::SupportFlags(support) => Request::SupportFlags(support)
        }
    }
}

impl TryFrom<Message> for Request {
    type Error = PeerError;

    /// Converts a monero message into an internal request,
    /// will return `PeerError::PeerSentUnSolicitedResponse` if 
    /// this is not a request
    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Response(_) => Err(PeerError::PeerSentUnSolicitedResponse),
            Message::Request(req) => Ok(req.into()),
            Message::Notification(noti) => {
                Ok(match *noti {
                    MessageNotification::RequestGetObject(obj) => Request::GetObjects(obj),
                    MessageNotification::RequestChain(chain) => Request::Chain(chain),
                    MessageNotification::RequestFluffyMissingTx(txs) => Request::FluffyMissingTx(txs),
                    MessageNotification::GetTxPoolComplement(comp) => Request::TxPoolCompliment(comp),
                    MessageNotification::NewBlock(block) => Request::NewBlock(block),
                    MessageNotification::NewFluffyBlock(block) => Request::NewFluffyBlock(block),
                    MessageNotification::NewTransactions(txs) => Request::NewTransactions(txs),
                    _ => return Err(PeerError::PeerSentUnSolicitedResponse)
                })
            }
        }
    }
}

/// A network message that will occur in response to a
/// request
#[derive(Debug, Clone)]
pub enum Response {
    Handshake(HandshakeResponse),
    TimedSync(TimedSyncResponse),
    Ping(PingResponse),
    SupportFlags(SupportFlagsResponse),
    GetObjects(GetObjectsResponse),
    Chain(ChainResponse),
    // these below could be a response or a "notification"
    NewFluffyBlock(NewFluffyBlock),
    NewTransactions(NewTransactions),
}

impl From<MessageResponse> for Response {
    fn from(value: MessageResponse) -> Self {
        match value {
            MessageResponse::Handshake(hand) => Response::Handshake(hand),
            MessageResponse::TimedSync(timed) => Response::TimedSync(timed),
            MessageResponse::Ping(ping) => Response::Ping(ping),
            MessageResponse::SupportFlags(support) => Response::SupportFlags(support)
        }
    }
}

#[derive(Debug)]
struct MessageToResponseError;

impl TryFrom<Message> for Response {
    type Error = MessageToResponseError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Response(res) => Ok(res.into()),
            Message::Request(req) => Err(MessageToResponseError),
            Message::Notification(noti) => {
                Ok(match *noti {
                    MessageNotification::ResponseGetObject(obj) => Response::GetObjects(obj),
                    MessageNotification::ResponseChainEntry(chain) => Response::Chain(chain),
                    MessageNotification::NewFluffyBlock(block) => Response::NewFluffyBlock(block),
                    MessageNotification::NewTransactions(txs) => Response::NewTransactions(txs),
                    _ => return Err(MessageToResponseError),
                })
            }
        }
    }
}

impl Into<Message> for Response {
    fn into(self) -> Message {
        match self {
            Response::Handshake(hand) => MessageResponse::Handshake(hand).into(),
            Response::TimedSync(timed) => MessageResponse::TimedSync(timed).into(),
            Response::Ping(ping) => MessageResponse::Ping(ping).into(),
            Response::SupportFlags(support) => MessageResponse::SupportFlags(support).into(),

            Response::GetObjects(obj) => MessageNotification::ResponseGetObject(obj).into(),
            Response::Chain(chain) => MessageNotification::ResponseChainEntry(chain).into(),
            Response::NewFluffyBlock(block) => MessageNotification::NewFluffyBlock(block).into(),
            Response::NewTransactions(txs) => MessageNotification::NewTransactions(txs).into(),


        }
    }
}