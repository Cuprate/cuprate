//! This module contains the implementations of [`TryFrom`] and [`From`] to convert between
//! [`Message`], [`PeerRequest`] and [`PeerResponse`].

use monero_wire::{Message, ProtocolMessage, RequestMessage, ResponseMessage};

use super::{PeerRequest, PeerResponse};

pub struct MessageConversionError;

macro_rules! match_body {
    (match $value: ident {$($body:tt)*} ($left:pat => $right_ty:expr) $($todo:tt)*) => {
        match_body!( match $value {
            $left => $right_ty,
            $($body)*
        } $($todo)* )
    };
    (match $value: ident {$($body:tt)*}) => {
         match $value {
            $($body)*
        }
    };
}

macro_rules! from {
    ($left_ty:ident, $right_ty:ident, {$($left:ident $(($val: ident))? = $right:ident $(($vall: ident))?,)+}) => {
        impl From<$left_ty> for $right_ty {
            fn from(value: $left_ty) -> Self {
                 match_body!( match value {}
                    $(($left_ty::$left$(($val))? => $right_ty::$right$(($vall))?))+
                )
            }
        }
    };
}

macro_rules! try_from {
    ($left_ty:ident, $right_ty:ident, {$($left:ident $(($val: ident))? = $right:ident $(($vall: ident))?,)+}) => {
        impl TryFrom<$left_ty> for $right_ty {
            type Error = MessageConversionError;

            fn try_from(value: $left_ty) -> Result<Self, Self::Error> {
                 Ok(match_body!( match value {
                        _ => return Err(MessageConversionError)
                    }
                    $(($left_ty::$left$(($val))? => $right_ty::$right$(($vall))?))+
                ))
            }
        }
    };
}

macro_rules! from_try_from {
    ($left_ty:ident, $right_ty:ident, {$($left:ident $(($val: ident))? = $right:ident $(($vall: ident))?,)+}) => {
        try_from!($left_ty, $right_ty, {$($left $(($val))? = $right $(($vall))?,)+});
        from!($right_ty, $left_ty, {$($right $(($val))? = $left $(($vall))?,)+});
    };
}

macro_rules! try_from_try_from {
    ($left_ty:ident, $right_ty:ident, {$($left:ident $(($val: ident))? = $right:ident $(($vall: ident))?,)+}) => {
        try_from!($left_ty, $right_ty, {$($left $(($val))? = $right $(($vall))?,)+});
        try_from!($right_ty, $left_ty, {$($right $(($val))? = $left $(($val))?,)+});
    };
}

from_try_from!(PeerRequest, RequestMessage,{
    Handshake(val) = Handshake(val),
    Ping = Ping,
    SupportFlags = SupportFlags,
    TimedSync(val) = TimedSync(val),
});

try_from_try_from!(PeerRequest, ProtocolMessage,{
    NewBlock(val) = NewBlock(val),
    NewFluffyBlock(val) = NewFluffyBlock(val),
    GetObjects(val) = GetObjectsRequest(val),
    GetChain(val) = ChainRequest(val),
    NewTransactions(val) = NewTransactions(val),
    FluffyMissingTxs(val) = FluffyMissingTransactionsRequest(val),
    GetTxPoolCompliment(val) = GetTxPoolCompliment(val),
});

impl TryFrom<Message> for PeerRequest {
    type Error = MessageConversionError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Request(req) => Ok(req.into()),
            Message::Protocol(pro) => pro.try_into(),
            _ => Err(MessageConversionError),
        }
    }
}

impl From<PeerRequest> for Message {
    fn from(value: PeerRequest) -> Self {
        match value {
            PeerRequest::Handshake(val) => Message::Request(RequestMessage::Handshake(val)),
            PeerRequest::Ping => Message::Request(RequestMessage::Ping),
            PeerRequest::SupportFlags => Message::Request(RequestMessage::SupportFlags),
            PeerRequest::TimedSync(val) => Message::Request(RequestMessage::TimedSync(val)),

            PeerRequest::NewBlock(val) => Message::Protocol(ProtocolMessage::NewBlock(val)),
            PeerRequest::NewFluffyBlock(val) => {
                Message::Protocol(ProtocolMessage::NewFluffyBlock(val))
            }
            PeerRequest::GetObjects(val) => {
                Message::Protocol(ProtocolMessage::GetObjectsRequest(val))
            }
            PeerRequest::GetChain(val) => Message::Protocol(ProtocolMessage::ChainRequest(val)),
            PeerRequest::NewTransactions(val) => {
                Message::Protocol(ProtocolMessage::NewTransactions(val))
            }
            PeerRequest::FluffyMissingTxs(val) => {
                Message::Protocol(ProtocolMessage::FluffyMissingTransactionsRequest(val))
            }
            PeerRequest::GetTxPoolCompliment(val) => {
                Message::Protocol(ProtocolMessage::GetTxPoolCompliment(val))
            }
        }
    }
}

from_try_from!(PeerResponse, ResponseMessage,{
    Handshake(val) = Handshake(val),
    Ping(val) = Ping(val),
    SupportFlags(val) = SupportFlags(val),
    TimedSync(val) = TimedSync(val),
});

try_from_try_from!(PeerResponse, ProtocolMessage,{
    NewFluffyBlock(val) = NewFluffyBlock(val),
    GetObjects(val) = GetObjectsResponse(val),
    GetChain(val) = ChainEntryResponse(val),
    NewTransactions(val) = NewTransactions(val),

});

impl TryFrom<Message> for PeerResponse {
    type Error = MessageConversionError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Response(res) => Ok(res.into()),
            Message::Protocol(pro) => pro.try_into(),
            _ => Err(MessageConversionError),
        }
    }
}

impl TryFrom<PeerResponse> for Message {
    type Error = MessageConversionError;

    fn try_from(value: PeerResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            PeerResponse::Handshake(val) => Message::Response(ResponseMessage::Handshake(val)),
            PeerResponse::Ping(val) => Message::Response(ResponseMessage::Ping(val)),
            PeerResponse::SupportFlags(val) => {
                Message::Response(ResponseMessage::SupportFlags(val))
            }
            PeerResponse::TimedSync(val) => Message::Response(ResponseMessage::TimedSync(val)),

            PeerResponse::NewFluffyBlock(val) => {
                Message::Protocol(ProtocolMessage::NewFluffyBlock(val))
            }
            PeerResponse::GetObjects(val) => {
                Message::Protocol(ProtocolMessage::GetObjectsResponse(val))
            }
            PeerResponse::GetChain(val) => {
                Message::Protocol(ProtocolMessage::ChainEntryResponse(val))
            }
            PeerResponse::NewTransactions(val) => {
                Message::Protocol(ProtocolMessage::NewTransactions(val))
            }

            PeerResponse::NA => return Err(MessageConversionError),
        })
    }
}
