// Rust Levin Library
// Written in 2023 by
//   Cuprate Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//

//! This module defines a Monero `Message` enum which contains
//! every possible Monero network message (levin body)

use std::fmt::Formatter;

use bytes::{Buf, BytesMut};

use cuprate_epee_encoding::epee_object;
use cuprate_levin::{
    BucketBuilder, BucketError, LevinBody, LevinCommand as LevinCommandTrait, MessageType,
};

pub mod admin;
pub mod common;
pub mod protocol;

use admin::*;
pub use common::{BasicNodeData, CoreSyncData, PeerListEntryBase};
use protocol::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LevinCommand {
    Handshake,
    TimedSync,
    Ping,
    SupportFlags,

    NewBlock,
    NewTransactions,
    GetObjectsRequest,
    GetObjectsResponse,
    ChainRequest,
    ChainResponse,
    NewFluffyBlock,
    FluffyMissingTxsRequest,
    GetTxPoolCompliment,

    Unknown(u32),
}

impl std::fmt::Display for LevinCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let LevinCommand::Unknown(id) = self {
            return f.write_str(&format!("unknown id: {}", id));
        }

        f.write_str(match self {
            LevinCommand::Handshake => "handshake",
            LevinCommand::TimedSync => "timed sync",
            LevinCommand::Ping => "ping",
            LevinCommand::SupportFlags => "support flags",

            LevinCommand::NewBlock => "new block",
            LevinCommand::NewTransactions => "new transactions",
            LevinCommand::GetObjectsRequest => "get objects request",
            LevinCommand::GetObjectsResponse => "get objects response",
            LevinCommand::ChainRequest => "chain request",
            LevinCommand::ChainResponse => "chain response",
            LevinCommand::NewFluffyBlock => "new fluffy block",
            LevinCommand::FluffyMissingTxsRequest => "fluffy missing transaction request",
            LevinCommand::GetTxPoolCompliment => "get transaction pool compliment",

            LevinCommand::Unknown(_) => unreachable!(),
        })
    }
}

impl LevinCommandTrait for LevinCommand {
    fn bucket_size_limit(&self) -> u64 {
        // https://github.com/monero-project/monero/blob/00fd416a99686f0956361d1cd0337fe56e58d4a7/src/cryptonote_basic/connection_context.cpp#L37
        match self {
            LevinCommand::Handshake => 65536,
            LevinCommand::TimedSync => 65536,
            LevinCommand::Ping => 4096,
            LevinCommand::SupportFlags => 4096,

            LevinCommand::NewBlock => 1024 * 1024 * 128, // 128 MB (max packet is a bit less than 100 MB though)
            LevinCommand::NewTransactions => 1024 * 1024 * 128, // 128 MB (max packet is a bit less than 100 MB though)
            LevinCommand::GetObjectsRequest => 1024 * 1024 * 2, // 2 MB
            LevinCommand::GetObjectsResponse => 1024 * 1024 * 128, // 128 MB (max packet is a bit less than 100 MB though)
            LevinCommand::ChainRequest => 512 * 1024,              // 512 kB
            LevinCommand::ChainResponse => 1024 * 1024 * 4,        // 4 MB
            LevinCommand::NewFluffyBlock => 1024 * 1024 * 4,       // 4 MB
            LevinCommand::FluffyMissingTxsRequest => 1024 * 1024,  // 1 MB
            LevinCommand::GetTxPoolCompliment => 1024 * 1024 * 4,  // 4 MB

            LevinCommand::Unknown(_) => usize::MAX.try_into().unwrap_or(u64::MAX),
        }
    }

    fn is_handshake(&self) -> bool {
        matches!(self, LevinCommand::Handshake)
    }
}

impl From<u32> for LevinCommand {
    fn from(value: u32) -> Self {
        match value {
            1001 => LevinCommand::Handshake,
            1002 => LevinCommand::TimedSync,
            1003 => LevinCommand::Ping,
            1007 => LevinCommand::SupportFlags,

            2001 => LevinCommand::NewBlock,
            2002 => LevinCommand::NewTransactions,
            2003 => LevinCommand::GetObjectsRequest,
            2004 => LevinCommand::GetObjectsResponse,
            2006 => LevinCommand::ChainRequest,
            2007 => LevinCommand::ChainResponse,
            2008 => LevinCommand::NewFluffyBlock,
            2009 => LevinCommand::FluffyMissingTxsRequest,
            2010 => LevinCommand::GetTxPoolCompliment,

            x => LevinCommand::Unknown(x),
        }
    }
}

impl From<LevinCommand> for u32 {
    fn from(value: LevinCommand) -> Self {
        match value {
            LevinCommand::Handshake => 1001,
            LevinCommand::TimedSync => 1002,
            LevinCommand::Ping => 1003,
            LevinCommand::SupportFlags => 1007,

            LevinCommand::NewBlock => 2001,
            LevinCommand::NewTransactions => 2002,
            LevinCommand::GetObjectsRequest => 2003,
            LevinCommand::GetObjectsResponse => 2004,
            LevinCommand::ChainRequest => 2006,
            LevinCommand::ChainResponse => 2007,
            LevinCommand::NewFluffyBlock => 2008,
            LevinCommand::FluffyMissingTxsRequest => 2009,
            LevinCommand::GetTxPoolCompliment => 2010,

            LevinCommand::Unknown(x) => x,
        }
    }
}

fn decode_message<B: Buf, T: cuprate_epee_encoding::EpeeObject, Ret>(
    ret: impl FnOnce(T) -> Ret,
    buf: &mut B,
) -> Result<Ret, BucketError> {
    let t = cuprate_epee_encoding::from_bytes(buf)
        .map_err(|e| BucketError::BodyDecodingError(e.into()))?;
    Ok(ret(t))
}

fn build_message<T: cuprate_epee_encoding::EpeeObject>(
    id: LevinCommand,
    val: T,
    builder: &mut BucketBuilder<LevinCommand>,
) -> Result<(), BucketError> {
    builder.set_command(id);
    builder.set_body(
        cuprate_epee_encoding::to_bytes(val)
            .map(BytesMut::freeze)
            .map_err(|e| BucketError::BodyDecodingError(e.into()))?,
    );
    Ok(())
}

pub enum ProtocolMessage {
    NewBlock(NewBlock),
    NewFluffyBlock(NewFluffyBlock),
    GetObjectsRequest(GetObjectsRequest),
    GetObjectsResponse(GetObjectsResponse),
    ChainRequest(ChainRequest),
    ChainEntryResponse(ChainResponse),
    NewTransactions(NewTransactions),
    FluffyMissingTransactionsRequest(FluffyMissingTransactionsRequest),
    GetTxPoolCompliment(GetTxPoolCompliment),
}

impl ProtocolMessage {
    pub fn command(&self) -> LevinCommand {
        use LevinCommand as C;

        match self {
            ProtocolMessage::NewBlock(_) => C::NewBlock,
            ProtocolMessage::NewFluffyBlock(_) => C::NewFluffyBlock,
            ProtocolMessage::GetObjectsRequest(_) => C::GetObjectsRequest,
            ProtocolMessage::GetObjectsResponse(_) => C::GetObjectsResponse,
            ProtocolMessage::ChainRequest(_) => C::ChainRequest,
            ProtocolMessage::ChainEntryResponse(_) => C::ChainResponse,
            ProtocolMessage::NewTransactions(_) => C::NewTransactions,
            ProtocolMessage::FluffyMissingTransactionsRequest(_) => C::FluffyMissingTxsRequest,
            ProtocolMessage::GetTxPoolCompliment(_) => C::GetTxPoolCompliment,
        }
    }

    fn decode<B: Buf>(buf: &mut B, command: LevinCommand) -> Result<Self, BucketError> {
        use LevinCommand as C;

        Ok(match command {
            C::NewBlock => decode_message(ProtocolMessage::NewBlock, buf)?,
            C::NewTransactions => decode_message(ProtocolMessage::NewTransactions, buf)?,
            C::GetObjectsRequest => decode_message(ProtocolMessage::GetObjectsRequest, buf)?,
            C::GetObjectsResponse => decode_message(ProtocolMessage::GetObjectsResponse, buf)?,
            C::ChainRequest => decode_message(ProtocolMessage::ChainRequest, buf)?,
            C::ChainResponse => decode_message(ProtocolMessage::ChainEntryResponse, buf)?,
            C::NewFluffyBlock => decode_message(ProtocolMessage::NewFluffyBlock, buf)?,
            C::FluffyMissingTxsRequest => {
                decode_message(ProtocolMessage::FluffyMissingTransactionsRequest, buf)?
            }
            C::GetTxPoolCompliment => decode_message(ProtocolMessage::GetTxPoolCompliment, buf)?,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        use LevinCommand as C;

        match self {
            ProtocolMessage::NewBlock(val) => build_message(C::NewBlock, val, builder)?,
            ProtocolMessage::NewTransactions(val) => {
                build_message(C::NewTransactions, val, builder)?
            }
            ProtocolMessage::GetObjectsRequest(val) => {
                build_message(C::GetObjectsRequest, val, builder)?
            }
            ProtocolMessage::GetObjectsResponse(val) => {
                build_message(C::GetObjectsResponse, val, builder)?
            }
            ProtocolMessage::ChainRequest(val) => build_message(C::ChainRequest, val, builder)?,
            ProtocolMessage::ChainEntryResponse(val) => {
                build_message(C::ChainResponse, val, builder)?
            }
            ProtocolMessage::NewFluffyBlock(val) => build_message(C::NewFluffyBlock, val, builder)?,
            ProtocolMessage::FluffyMissingTransactionsRequest(val) => {
                build_message(C::FluffyMissingTxsRequest, val, builder)?
            }
            ProtocolMessage::GetTxPoolCompliment(val) => {
                build_message(C::GetTxPoolCompliment, val, builder)?
            }
        }
        Ok(())
    }
}

pub enum RequestMessage {
    Handshake(HandshakeRequest),
    Ping,
    SupportFlags,
    TimedSync(TimedSyncRequest),
}

impl RequestMessage {
    pub fn command(&self) -> LevinCommand {
        use LevinCommand as C;

        match self {
            RequestMessage::Handshake(_) => C::Handshake,
            RequestMessage::Ping => C::Ping,
            RequestMessage::SupportFlags => C::SupportFlags,
            RequestMessage::TimedSync(_) => C::TimedSync,
        }
    }

    fn decode<B: Buf>(buf: &mut B, command: LevinCommand) -> Result<Self, BucketError> {
        use LevinCommand as C;

        Ok(match command {
            C::Handshake => decode_message(RequestMessage::Handshake, buf)?,
            C::TimedSync => decode_message(RequestMessage::TimedSync, buf)?,
            C::Ping => {
                cuprate_epee_encoding::from_bytes::<EmptyMessage, _>(buf)
                    .map_err(|e| BucketError::BodyDecodingError(e.into()))?;

                RequestMessage::Ping
            }
            C::SupportFlags => {
                cuprate_epee_encoding::from_bytes::<EmptyMessage, _>(buf)
                    .map_err(|e| BucketError::BodyDecodingError(e.into()))?;

                RequestMessage::SupportFlags
            }
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        use LevinCommand as C;

        match self {
            RequestMessage::Handshake(val) => build_message(C::Handshake, val, builder)?,
            RequestMessage::TimedSync(val) => build_message(C::TimedSync, val, builder)?,
            RequestMessage::Ping => build_message(C::Ping, EmptyMessage, builder)?,
            RequestMessage::SupportFlags => build_message(C::SupportFlags, EmptyMessage, builder)?,
        }
        Ok(())
    }
}

pub enum ResponseMessage {
    Handshake(HandshakeResponse),
    Ping(PingResponse),
    SupportFlags(SupportFlagsResponse),
    TimedSync(TimedSyncResponse),
}

impl ResponseMessage {
    pub fn command(&self) -> LevinCommand {
        use LevinCommand as C;

        match self {
            ResponseMessage::Handshake(_) => C::Handshake,
            ResponseMessage::Ping(_) => C::Ping,
            ResponseMessage::SupportFlags(_) => C::SupportFlags,
            ResponseMessage::TimedSync(_) => C::TimedSync,
        }
    }

    fn decode<B: Buf>(buf: &mut B, command: LevinCommand) -> Result<Self, BucketError> {
        use LevinCommand as C;

        Ok(match command {
            C::Handshake => decode_message(ResponseMessage::Handshake, buf)?,
            C::TimedSync => decode_message(ResponseMessage::TimedSync, buf)?,
            C::Ping => decode_message(ResponseMessage::Ping, buf)?,
            C::SupportFlags => decode_message(ResponseMessage::SupportFlags, buf)?,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        use LevinCommand as C;

        match self {
            ResponseMessage::Handshake(val) => build_message(C::Handshake, val, builder)?,
            ResponseMessage::TimedSync(val) => build_message(C::TimedSync, val, builder)?,
            ResponseMessage::Ping(val) => build_message(C::Ping, val, builder)?,
            ResponseMessage::SupportFlags(val) => build_message(C::SupportFlags, val, builder)?,
        }
        Ok(())
    }
}

pub enum Message {
    Request(RequestMessage),
    Response(ResponseMessage),
    Protocol(ProtocolMessage),
}

impl Message {
    pub fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, Message::Response(_))
    }

    pub fn is_protocol(&self) -> bool {
        matches!(self, Message::Protocol(_))
    }

    pub fn command(&self) -> LevinCommand {
        match self {
            Message::Request(mes) => mes.command(),
            Message::Response(mes) => mes.command(),
            Message::Protocol(mes) => mes.command(),
        }
    }
}

impl LevinBody for Message {
    type Command = LevinCommand;

    fn decode_message<B: Buf>(
        body: &mut B,
        typ: MessageType,
        command: LevinCommand,
    ) -> Result<Self, BucketError> {
        Ok(match typ {
            MessageType::Request => Message::Request(RequestMessage::decode(body, command)?),
            MessageType::Response => Message::Response(ResponseMessage::decode(body, command)?),
            MessageType::Notification => Message::Protocol(ProtocolMessage::decode(body, command)?),
        })
    }

    fn encode(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        match self {
            Message::Protocol(pro) => {
                builder.set_message_type(MessageType::Notification);
                builder.set_return_code(0);
                pro.build(builder)
            }
            Message::Request(req) => {
                builder.set_message_type(MessageType::Request);
                builder.set_return_code(0);
                req.build(builder)
            }
            Message::Response(res) => {
                builder.set_message_type(MessageType::Response);
                builder.set_return_code(1);
                res.build(builder)
            }
        }
    }
}

/// An internal empty message.
///
/// This represents P2P messages that have no fields as epee's binary format will still add a header
/// for these objects, so we need to decode/encode a message.
struct EmptyMessage;

epee_object! {
    EmptyMessage,
}
