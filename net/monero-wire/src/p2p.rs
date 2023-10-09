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

use levin_cuprate::{BucketBuilder, BucketError, LevinBody, MessageType};

pub mod admin;
pub mod common;
pub mod protocol;

use admin::*;
pub use common::{BasicNodeData, CoreSyncData, PeerListEntryBase};
use protocol::*;

fn decode_message<T: serde::de::DeserializeOwned, Ret>(
    ret: impl FnOnce(T) -> Ret,
    buf: &[u8],
) -> Result<Ret, BucketError> {
    let t = monero_epee_bin_serde::from_bytes(buf)
        .map_err(|e| BucketError::BodyDecodingError(e.into()))?;
    Ok(ret(t))
}

fn build_message<T: serde::Serialize>(
    id: u32,
    val: &T,
    builder: &mut BucketBuilder,
) -> Result<(), BucketError> {
    builder.set_command(id);
    builder.set_body(
        monero_epee_bin_serde::to_bytes(val)
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
    fn decode(buf: &[u8], command: u32) -> Result<Self, BucketError> {
        Ok(match command {
            2001 => decode_message(ProtocolMessage::NewBlock, buf)?,
            2002 => decode_message(ProtocolMessage::NewTransactions, buf)?,
            2003 => decode_message(ProtocolMessage::GetObjectsRequest, buf)?,
            2004 => decode_message(ProtocolMessage::GetObjectsResponse, buf)?,
            2006 => decode_message(ProtocolMessage::ChainRequest, buf)?,
            2007 => decode_message(ProtocolMessage::ChainEntryResponse, buf)?,
            2008 => decode_message(ProtocolMessage::NewFluffyBlock, buf)?,
            2009 => decode_message(ProtocolMessage::FluffyMissingTransactionsRequest, buf)?,
            2010 => decode_message(ProtocolMessage::GetTxPoolCompliment, buf)?,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(&self, builder: &mut BucketBuilder) -> Result<(), BucketError> {
        match self {
            ProtocolMessage::NewBlock(val) => build_message(2001, val, builder)?,
            ProtocolMessage::NewTransactions(val) => build_message(2002, val, builder)?,
            ProtocolMessage::GetObjectsRequest(val) => build_message(2003, val, builder)?,
            ProtocolMessage::GetObjectsResponse(val) => build_message(2004, val, builder)?,
            ProtocolMessage::ChainRequest(val) => build_message(2006, val, builder)?,
            ProtocolMessage::ChainEntryResponse(val) => build_message(2007, &val, builder)?,
            ProtocolMessage::NewFluffyBlock(val) => build_message(2008, val, builder)?,
            ProtocolMessage::FluffyMissingTransactionsRequest(val) => {
                build_message(2009, val, builder)?
            }
            ProtocolMessage::GetTxPoolCompliment(val) => build_message(2010, val, builder)?,
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
    fn decode(buf: &[u8], command: u32) -> Result<Self, BucketError> {
        Ok(match command {
            1001 => decode_message(RequestMessage::Handshake, buf)?,
            1002 => decode_message(RequestMessage::TimedSync, buf)?,
            1003 => RequestMessage::Ping,
            1007 => RequestMessage::SupportFlags,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(&self, builder: &mut BucketBuilder) -> Result<(), BucketError> {
        match self {
            RequestMessage::Handshake(val) => build_message(1001, val, builder)?,
            RequestMessage::TimedSync(val) => build_message(1002, val, builder)?,
            RequestMessage::Ping => {
                builder.set_command(1003);
                builder.set_body(Vec::new());
            }
            RequestMessage::SupportFlags => {
                builder.set_command(1007);
                builder.set_body(Vec::new());
            }
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
    fn decode(buf: &[u8], command: u32) -> Result<Self, BucketError> {
        Ok(match command {
            1001 => decode_message(ResponseMessage::Handshake, buf)?,
            1002 => decode_message(ResponseMessage::TimedSync, buf)?,
            1003 => decode_message(ResponseMessage::Ping, buf)?,
            1007 => decode_message(ResponseMessage::SupportFlags, buf)?,
            _ => return Err(BucketError::UnknownCommand),
        })
    }

    fn build(&self, builder: &mut BucketBuilder) -> Result<(), BucketError> {
        match self {
            ResponseMessage::Handshake(val) => build_message(1001, val, builder)?,
            ResponseMessage::TimedSync(val) => build_message(1002, val, builder)?,
            ResponseMessage::Ping(val) => build_message(1003, val, builder)?,
            ResponseMessage::SupportFlags(val) => build_message(1007, val, builder)?,
        }
        Ok(())
    }
}

pub enum Message {
    Request(RequestMessage),
    Response(ResponseMessage),
    Protocol(ProtocolMessage),
}

impl LevinBody for Message {
    fn decode_message(body: &[u8], typ: MessageType, command: u32) -> Result<Self, BucketError> {
        Ok(match typ {
            MessageType::Request => Message::Request(RequestMessage::decode(body, command)?),
            MessageType::Response => Message::Response(ResponseMessage::decode(body, command)?),
            MessageType::Notification => Message::Protocol(ProtocolMessage::decode(body, command)?),
        })
    }

    fn encode(&self, builder: &mut BucketBuilder) -> Result<(), BucketError> {
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
