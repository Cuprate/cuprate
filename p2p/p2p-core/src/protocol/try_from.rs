//! This module contains the implementations of [`TryFrom`] and [`From`] to convert between
//! [`Message`], [`PeerRequest`] and [`PeerResponse`].

use cuprate_wire::{Message, ProtocolMessage};

use crate::{PeerRequest, PeerResponse, ProtocolRequest, ProtocolResponse};

#[derive(Debug)]
pub struct MessageConversionError;

impl From<ProtocolRequest> for ProtocolMessage {
    fn from(value: ProtocolRequest) -> Self {
        match value {
            ProtocolRequest::GetObjects(val) => ProtocolMessage::GetObjectsRequest(val),
            ProtocolRequest::GetChain(val) => ProtocolMessage::ChainRequest(val),
            ProtocolRequest::FluffyMissingTxs(val) => {
                ProtocolMessage::FluffyMissingTransactionsRequest(val)
            }
            ProtocolRequest::GetTxPoolCompliment(val) => ProtocolMessage::GetTxPoolCompliment(val),
            ProtocolRequest::NewBlock(val) => ProtocolMessage::NewBlock(val),
            ProtocolRequest::NewFluffyBlock(val) => ProtocolMessage::NewFluffyBlock(val),
            ProtocolRequest::NewTransactions(val) => ProtocolMessage::NewTransactions(val),
        }
    }
}

impl TryFrom<ProtocolMessage> for ProtocolRequest {
    type Error = MessageConversionError;

    fn try_from(value: ProtocolMessage) -> Result<Self, Self::Error> {
        Ok(match value {
            ProtocolMessage::GetObjectsRequest(val) => ProtocolRequest::GetObjects(val),
            ProtocolMessage::ChainRequest(val) => ProtocolRequest::GetChain(val),
            ProtocolMessage::FluffyMissingTransactionsRequest(val) => {
                ProtocolRequest::FluffyMissingTxs(val)
            }
            ProtocolMessage::GetTxPoolCompliment(val) => ProtocolRequest::GetTxPoolCompliment(val),
            ProtocolMessage::NewBlock(val) => ProtocolRequest::NewBlock(val),
            ProtocolMessage::NewFluffyBlock(val) => ProtocolRequest::NewFluffyBlock(val),
            ProtocolMessage::NewTransactions(val) => ProtocolRequest::NewTransactions(val),
            ProtocolMessage::GetObjectsResponse(_) | ProtocolMessage::ChainEntryResponse(_) => {
                return Err(MessageConversionError)
            }
        })
    }
}

impl From<PeerRequest> for Message {
    fn from(value: PeerRequest) -> Self {
        match value {
            PeerRequest::Admin(val) => Message::Request(val),
            PeerRequest::Protocol(val) => Message::Protocol(val.into()),
        }
    }
}

impl TryFrom<Message> for PeerRequest {
    type Error = MessageConversionError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Request(req) => Ok(PeerRequest::Admin(req)),
            Message::Protocol(pro) => Ok(PeerRequest::Protocol(pro.try_into()?)),
            Message::Response(_) => Err(MessageConversionError),
        }
    }
}

impl TryFrom<ProtocolResponse> for ProtocolMessage {
    type Error = MessageConversionError;

    fn try_from(value: ProtocolResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            ProtocolResponse::NewTransactions(val) => ProtocolMessage::NewTransactions(val),
            ProtocolResponse::NewFluffyBlock(val) => ProtocolMessage::NewFluffyBlock(val),
            ProtocolResponse::GetChain(val) => ProtocolMessage::ChainEntryResponse(val),
            ProtocolResponse::GetObjects(val) => ProtocolMessage::GetObjectsResponse(val),
            ProtocolResponse::NA => return Err(MessageConversionError),
        })
    }
}

impl TryFrom<ProtocolMessage> for ProtocolResponse {
    type Error = MessageConversionError;

    fn try_from(value: ProtocolMessage) -> Result<Self, Self::Error> {
        Ok(match value {
            ProtocolMessage::NewTransactions(val) => ProtocolResponse::NewTransactions(val),
            ProtocolMessage::NewFluffyBlock(val) => ProtocolResponse::NewFluffyBlock(val),
            ProtocolMessage::ChainEntryResponse(val) => ProtocolResponse::GetChain(val),
            ProtocolMessage::GetObjectsResponse(val) => ProtocolResponse::GetObjects(val),
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
            Message::Response(res) => Ok(PeerResponse::Admin(res)),
            Message::Protocol(pro) => Ok(PeerResponse::Protocol(pro.try_into()?)),
            Message::Request(_) => Err(MessageConversionError),
        }
    }
}

impl TryFrom<PeerResponse> for Message {
    type Error = MessageConversionError;

    fn try_from(value: PeerResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            PeerResponse::Admin(val) => Message::Response(val),
            PeerResponse::Protocol(val) => Message::Protocol(val.try_into()?),
        })
    }
}
