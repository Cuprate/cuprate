//! This module contains the implementations of [`TryFrom`] and [`From`] to convert between
//! [`Message`], [`PeerRequest`] and [`PeerResponse`].

use cuprate_wire::{Message, ProtocolMessage};

use crate::{BroadcastMessage, PeerRequest, PeerResponse, ProtocolRequest, ProtocolResponse};

#[derive(Debug)]
pub struct MessageConversionError;

impl From<ProtocolRequest> for ProtocolMessage {
    fn from(value: ProtocolRequest) -> Self {
        match value {
            ProtocolRequest::GetObjects(val) => Self::GetObjectsRequest(val),
            ProtocolRequest::GetChain(val) => Self::ChainRequest(val),
            ProtocolRequest::FluffyMissingTxs(val) => Self::FluffyMissingTransactionsRequest(val),
            ProtocolRequest::GetTxPoolCompliment(val) => Self::GetTxPoolCompliment(val),
            ProtocolRequest::NewBlock(val) => Self::NewBlock(val),
            ProtocolRequest::NewFluffyBlock(val) => Self::NewFluffyBlock(val),
            ProtocolRequest::NewTransactions(val) => Self::NewTransactions(val),
        }
    }
}

impl TryFrom<ProtocolMessage> for ProtocolRequest {
    type Error = MessageConversionError;

    fn try_from(value: ProtocolMessage) -> Result<Self, Self::Error> {
        Ok(match value {
            ProtocolMessage::GetObjectsRequest(val) => Self::GetObjects(val),
            ProtocolMessage::ChainRequest(val) => Self::GetChain(val),
            ProtocolMessage::FluffyMissingTransactionsRequest(val) => Self::FluffyMissingTxs(val),
            ProtocolMessage::GetTxPoolCompliment(val) => Self::GetTxPoolCompliment(val),
            ProtocolMessage::NewBlock(val) => Self::NewBlock(val),
            ProtocolMessage::NewFluffyBlock(val) => Self::NewFluffyBlock(val),
            ProtocolMessage::NewTransactions(val) => Self::NewTransactions(val),
            ProtocolMessage::GetObjectsResponse(_) | ProtocolMessage::ChainEntryResponse(_) => {
                return Err(MessageConversionError)
            }
        })
    }
}

impl From<PeerRequest> for Message {
    fn from(value: PeerRequest) -> Self {
        match value {
            PeerRequest::Admin(val) => Self::Request(val),
            PeerRequest::Protocol(val) => Self::Protocol(val.into()),
        }
    }
}

impl TryFrom<Message> for PeerRequest {
    type Error = MessageConversionError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Request(req) => Ok(Self::Admin(req)),
            Message::Protocol(pro) => Ok(Self::Protocol(pro.try_into()?)),
            Message::Response(_) => Err(MessageConversionError),
        }
    }
}

impl TryFrom<ProtocolResponse> for ProtocolMessage {
    type Error = MessageConversionError;

    fn try_from(value: ProtocolResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            ProtocolResponse::NewTransactions(val) => Self::NewTransactions(val),
            ProtocolResponse::NewFluffyBlock(val) => Self::NewFluffyBlock(val),
            ProtocolResponse::GetChain(val) => Self::ChainEntryResponse(val),
            ProtocolResponse::GetObjects(val) => Self::GetObjectsResponse(val),
            ProtocolResponse::FluffyMissingTransactionsRequest(val) => {
                Self::FluffyMissingTransactionsRequest(val)
            }
            ProtocolResponse::NA => return Err(MessageConversionError),
        })
    }
}

impl TryFrom<ProtocolMessage> for ProtocolResponse {
    type Error = MessageConversionError;

    fn try_from(value: ProtocolMessage) -> Result<Self, Self::Error> {
        Ok(match value {
            ProtocolMessage::NewTransactions(val) => Self::NewTransactions(val),
            ProtocolMessage::NewFluffyBlock(val) => Self::NewFluffyBlock(val),
            ProtocolMessage::ChainEntryResponse(val) => Self::GetChain(val),
            ProtocolMessage::GetObjectsResponse(val) => Self::GetObjects(val),
            ProtocolMessage::ChainRequest(_)
            | ProtocolMessage::FluffyMissingTransactionsRequest(_)
            | ProtocolMessage::GetObjectsRequest(_)
            | ProtocolMessage::GetTxPoolCompliment(_)
            | ProtocolMessage::NewBlock(_) => return Err(MessageConversionError),
        })
    }
}

impl TryFrom<Message> for PeerResponse {
    type Error = MessageConversionError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Response(res) => Ok(Self::Admin(res)),
            Message::Protocol(pro) => Ok(Self::Protocol(pro.try_into()?)),
            Message::Request(_) => Err(MessageConversionError),
        }
    }
}

impl TryFrom<PeerResponse> for Message {
    type Error = MessageConversionError;

    fn try_from(value: PeerResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            PeerResponse::Admin(val) => Self::Response(val),
            PeerResponse::Protocol(val) => Self::Protocol(val.try_into()?),
        })
    }
}

impl TryFrom<PeerRequest> for BroadcastMessage {
    type Error = MessageConversionError;

    fn try_from(value: PeerRequest) -> Result<Self, Self::Error> {
        match value {
            PeerRequest::Protocol(ProtocolRequest::NewTransactions(txs)) => {
                Ok(Self::NewTransactions(txs))
            }
            PeerRequest::Protocol(ProtocolRequest::NewFluffyBlock(block)) => {
                Ok(Self::NewFluffyBlock(block))
            }
            PeerRequest::Admin(_) | PeerRequest::Protocol(_) => Err(MessageConversionError),
        }
    }
}

impl From<BroadcastMessage> for PeerRequest {
    fn from(value: BroadcastMessage) -> Self {
        match value {
            BroadcastMessage::NewTransactions(txs) => {
                Self::Protocol(ProtocolRequest::NewTransactions(txs))
            }
            BroadcastMessage::NewFluffyBlock(block) => {
                Self::Protocol(ProtocolRequest::NewFluffyBlock(block))
            }
        }
    }
}
