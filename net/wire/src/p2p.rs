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
    TxPoolInv,
    RequestTxPoolTxs,

    Unknown(u32),
}

impl std::fmt::Display for LevinCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Self::Unknown(id) = self {
            return f.write_str(&format!("unknown id: {id}"));
        }

        f.write_str(match self {
            Self::Handshake => "handshake",
            Self::TimedSync => "timed sync",
            Self::Ping => "ping",
            Self::SupportFlags => "support flags",

            Self::NewBlock => "new block",
            Self::NewTransactions => "new transactions",
            Self::GetObjectsRequest => "get objects request",
            Self::GetObjectsResponse => "get objects response",
            Self::ChainRequest => "chain request",
            Self::ChainResponse => "chain response",
            Self::NewFluffyBlock => "new fluffy block",
            Self::FluffyMissingTxsRequest => "fluffy missing transaction request",
            Self::GetTxPoolCompliment => "get transaction pool compliment",
            Self::TxPoolInv => "tx pool inv",
            Self::RequestTxPoolTxs => "request tx pool txs",

            Self::Unknown(_) => unreachable!(),
        })
    }
}

impl LevinCommandTrait for LevinCommand {
    fn bucket_size_limit(&self) -> u64 {
        // https://github.com/monero-project/monero/blob/00fd416a99686f0956361d1cd0337fe56e58d4a7/src/cryptonote_basic/connection_context.cpp#L37
        #[expect(clippy::match_same_arms, reason = "formatting is more clear")]
        match self {
            Self::Handshake => 65536,
            Self::TimedSync => 65536,
            Self::Ping => 4096,
            Self::SupportFlags => 4096,

            Self::NewBlock => 1024 * 1024 * 128, // 128 MB (max packet is a bit less than 100 MB though)
            Self::NewTransactions => 1024 * 1024 * 128, // 128 MB (max packet is a bit less than 100 MB though)
            Self::GetObjectsRequest => 1024 * 1024 * 2, // 2 MB
            Self::GetObjectsResponse => 1024 * 1024 * 128, // 128 MB (max packet is a bit less than 100 MB though)
            Self::ChainRequest => 512 * 1024,              // 512 kB
            Self::ChainResponse => 1024 * 1024 * 4,        // 4 MB
            Self::NewFluffyBlock => 1024 * 1024 * 4,       // 4 MB
            Self::FluffyMissingTxsRequest => 1024 * 1024,  // 1 MB
            Self::GetTxPoolCompliment => 1024 * 1024 * 4,  // 4 MB
            Self::TxPoolInv => 512 * 1024, //  512 kB
            Self::RequestTxPoolTxs => 512 * 1024, //  512 kB

            Self::Unknown(_) => u64::MAX,
        }
    }

    fn is_handshake(&self) -> bool {
        matches!(self, Self::Handshake)
    }
}

impl From<u32> for LevinCommand {
    fn from(value: u32) -> Self {
        match value {
            1001 => Self::Handshake,
            1002 => Self::TimedSync,
            1003 => Self::Ping,
            1007 => Self::SupportFlags,

            2001 => Self::NewBlock,
            2002 => Self::NewTransactions,
            2003 => Self::GetObjectsRequest,
            2004 => Self::GetObjectsResponse,
            2006 => Self::ChainRequest,
            2007 => Self::ChainResponse,
            2008 => Self::NewFluffyBlock,
            2009 => Self::FluffyMissingTxsRequest,
            2010 => Self::GetTxPoolCompliment,
            2011 => Self::TxPoolInv,
            2012 => Self::RequestTxPoolTxs,

            x => Self::Unknown(x),
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
            LevinCommand::TxPoolInv => 2011,
            LevinCommand::RequestTxPoolTxs => 2012,

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

#[derive(Debug, Clone)]
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
    TxPoolInv(TxPoolInv),
    RequestTxPoolTxs(RequestTxPoolTxs),
}

impl ProtocolMessage {
    pub const fn command(&self) -> LevinCommand {
        use LevinCommand as C;

        match self {
            Self::NewBlock(_) => C::NewBlock,
            Self::NewFluffyBlock(_) => C::NewFluffyBlock,
            Self::GetObjectsRequest(_) => C::GetObjectsRequest,
            Self::GetObjectsResponse(_) => C::GetObjectsResponse,
            Self::ChainRequest(_) => C::ChainRequest,
            Self::ChainEntryResponse(_) => C::ChainResponse,
            Self::NewTransactions(_) => C::NewTransactions,
            Self::FluffyMissingTransactionsRequest(_) => C::FluffyMissingTxsRequest,
            Self::GetTxPoolCompliment(_) => C::GetTxPoolCompliment,
            Self::TxPoolInv(_) => C::TxPoolInv,
            Self::RequestTxPoolTxs(_) => C::RequestTxPoolTxs,
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
            C::TxPoolInv => decode_message(ProtocolMessage::TxPoolInv, buf)?,
            C::RequestTxPoolTxs => decode_message(ProtocolMessage::RequestTxPoolTxs, buf)?,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        use LevinCommand as C;

        match self {
            Self::NewBlock(val) => build_message(C::NewBlock, val, builder)?,
            Self::NewTransactions(val) => {
                build_message(C::NewTransactions, val, builder)?;
            }
            Self::GetObjectsRequest(val) => {
                build_message(C::GetObjectsRequest, val, builder)?;
            }
            Self::GetObjectsResponse(val) => {
                build_message(C::GetObjectsResponse, val, builder)?;
            }
            Self::ChainRequest(val) => build_message(C::ChainRequest, val, builder)?,
            Self::ChainEntryResponse(val) => {
                build_message(C::ChainResponse, val, builder)?;
            }
            Self::NewFluffyBlock(val) => build_message(C::NewFluffyBlock, val, builder)?,
            Self::FluffyMissingTransactionsRequest(val) => {
                build_message(C::FluffyMissingTxsRequest, val, builder)?;
            }
            Self::GetTxPoolCompliment(val) => {
                build_message(C::GetTxPoolCompliment, val, builder)?;
            }
            ProtocolMessage::TxPoolInv(val) => build_message(C::TxPoolInv, val, builder)?,
            ProtocolMessage::RequestTxPoolTxs(val) => build_message(C::RequestTxPoolTxs, val, builder)?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum AdminRequestMessage {
    Handshake(HandshakeRequest),
    Ping,
    SupportFlags,
    TimedSync(TimedSyncRequest),
}

impl AdminRequestMessage {
    pub const fn command(&self) -> LevinCommand {
        use LevinCommand as C;

        match self {
            Self::Handshake(_) => C::Handshake,
            Self::Ping => C::Ping,
            Self::SupportFlags => C::SupportFlags,
            Self::TimedSync(_) => C::TimedSync,
        }
    }

    fn decode<B: Buf>(buf: &mut B, command: LevinCommand) -> Result<Self, BucketError> {
        use LevinCommand as C;

        Ok(match command {
            C::Handshake => decode_message(AdminRequestMessage::Handshake, buf)?,
            C::TimedSync => decode_message(AdminRequestMessage::TimedSync, buf)?,
            C::Ping => {
                cuprate_epee_encoding::from_bytes::<EmptyMessage, _>(buf)
                    .map_err(|e| BucketError::BodyDecodingError(e.into()))?;

                Self::Ping
            }
            C::SupportFlags => {
                cuprate_epee_encoding::from_bytes::<EmptyMessage, _>(buf)
                    .map_err(|e| BucketError::BodyDecodingError(e.into()))?;

                Self::SupportFlags
            }
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        use LevinCommand as C;

        match self {
            Self::Handshake(val) => build_message(C::Handshake, val, builder)?,
            Self::TimedSync(val) => build_message(C::TimedSync, val, builder)?,
            Self::Ping => build_message(C::Ping, EmptyMessage, builder)?,
            Self::SupportFlags => {
                build_message(C::SupportFlags, EmptyMessage, builder)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum AdminResponseMessage {
    Handshake(HandshakeResponse),
    Ping(PingResponse),
    SupportFlags(SupportFlagsResponse),
    TimedSync(TimedSyncResponse),
}

impl AdminResponseMessage {
    pub const fn command(&self) -> LevinCommand {
        use LevinCommand as C;

        match self {
            Self::Handshake(_) => C::Handshake,
            Self::Ping(_) => C::Ping,
            Self::SupportFlags(_) => C::SupportFlags,
            Self::TimedSync(_) => C::TimedSync,
        }
    }

    fn decode<B: Buf>(buf: &mut B, command: LevinCommand) -> Result<Self, BucketError> {
        use LevinCommand as C;

        Ok(match command {
            C::Handshake => decode_message(AdminResponseMessage::Handshake, buf)?,
            C::TimedSync => decode_message(AdminResponseMessage::TimedSync, buf)?,
            C::Ping => decode_message(AdminResponseMessage::Ping, buf)?,
            C::SupportFlags => decode_message(AdminResponseMessage::SupportFlags, buf)?,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        use LevinCommand as C;

        match self {
            Self::Handshake(val) => build_message(C::Handshake, val, builder)?,
            Self::TimedSync(val) => build_message(C::TimedSync, val, builder)?,
            Self::Ping(val) => build_message(C::Ping, val, builder)?,
            Self::SupportFlags(val) => {
                build_message(C::SupportFlags, val, builder)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Request(AdminRequestMessage),
    Response(AdminResponseMessage),
    Protocol(ProtocolMessage),
}

impl Message {
    pub const fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    pub const fn is_response(&self) -> bool {
        matches!(self, Self::Response(_))
    }

    pub const fn is_protocol(&self) -> bool {
        matches!(self, Self::Protocol(_))
    }

    pub const fn command(&self) -> LevinCommand {
        match self {
            Self::Request(mes) => mes.command(),
            Self::Response(mes) => mes.command(),
            Self::Protocol(mes) => mes.command(),
        }
    }
}

impl LevinBody for Message {
    type Command = LevinCommand;

    fn decode_message<B: Buf>(
        body: &mut B,
        ty: MessageType,
        command: LevinCommand,
    ) -> Result<Self, BucketError> {
        Ok(match ty {
            MessageType::Request => Self::Request(AdminRequestMessage::decode(body, command)?),
            MessageType::Response => Self::Response(AdminResponseMessage::decode(body, command)?),
            MessageType::Notification => Self::Protocol(ProtocolMessage::decode(body, command)?),
        })
    }

    fn encode(self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
        match self {
            Self::Protocol(pro) => {
                builder.set_message_type(MessageType::Notification);
                builder.set_return_code(0);
                pro.build(builder)
            }
            Self::Request(req) => {
                builder.set_message_type(MessageType::Request);
                builder.set_return_code(0);
                req.build(builder)
            }
            Self::Response(res) => {
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
