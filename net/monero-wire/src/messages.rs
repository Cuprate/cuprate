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

pub use admin::{
    HandshakeRequest, HandshakeResponse, PingResponse, SupportFlagsResponse, TimedSyncRequest,
    TimedSyncResponse,
};
pub use common::{BasicNodeData, CoreSyncData, PeerListEntryBase};
pub use protocol::{
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
    GetObjectsResponse, GetTxPoolCompliment, NewBlock, NewFluffyBlock, NewTransactions,
};

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
    fn decode(buf: &[u8], command: u32) -> Result<Self, epee_encoding::Error> {
        Ok(match command {
            2001 => ProtocolMessage::NewBlock(epee_encoding::from_bytes(buf)?),
            2002 => ProtocolMessage::NewTransactions(epee_encoding::from_bytes(buf)?),
            2003 => ProtocolMessage::GetObjectsRequest(epee_encoding::from_bytes(buf)?),
            2004 => ProtocolMessage::GetObjectsResponse(epee_encoding::from_bytes(buf)?),
            2006 => ProtocolMessage::ChainRequest(epee_encoding::from_bytes(buf)?),
            2007 => ProtocolMessage::ChainEntryResponse(epee_encoding::from_bytes(buf)?),
            2008 => ProtocolMessage::NewFluffyBlock(epee_encoding::from_bytes(buf)?),
            2009 => {
                ProtocolMessage::FluffyMissingTransactionsRequest(epee_encoding::from_bytes(buf)?)
            }
            2010 => ProtocolMessage::GetTxPoolCompliment(epee_encoding::from_bytes(buf)?),
            _ => {
                return Err(epee_encoding::Error::Value(
                    "Failed to decode message, unknown command",
                ))
            }
        })
    }

    fn build(&self, builder: &mut BucketBuilder) -> Result<(), epee_encoding::Error> {
        match self {
            ProtocolMessage::NewBlock(nb) => {
                builder.set_command(2001);
                builder.set_body(epee_encoding::to_bytes(nb)?);
            }
            ProtocolMessage::NewTransactions(nt) => {
                builder.set_command(2002);
                builder.set_body(epee_encoding::to_bytes(nt)?);
            }
            ProtocolMessage::GetObjectsRequest(gt) => {
                builder.set_command(2003);
                builder.set_body(epee_encoding::to_bytes(gt)?);
            }
            ProtocolMessage::GetObjectsResponse(ge) => {
                builder.set_command(2004);
                builder.set_body(epee_encoding::to_bytes(ge)?);
            }
            ProtocolMessage::ChainRequest(ct) => {
                builder.set_command(2006);
                builder.set_body(epee_encoding::to_bytes(ct)?);
            }
            ProtocolMessage::ChainEntryResponse(ce) => {
                builder.set_command(2007);
                builder.set_body(epee_encoding::to_bytes(ce)?);
            }
            ProtocolMessage::NewFluffyBlock(fb) => {
                builder.set_command(2008);
                builder.set_body(epee_encoding::to_bytes(fb)?);
            }
            ProtocolMessage::FluffyMissingTransactionsRequest(ft) => {
                builder.set_command(2009);
                builder.set_body(epee_encoding::to_bytes(ft)?);
            }
            ProtocolMessage::GetTxPoolCompliment(tp) => {
                builder.set_command(2010);
                builder.set_body(epee_encoding::to_bytes(tp)?);
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
    fn decode(buf: &[u8], command: u32) -> Result<Self, epee_encoding::Error> {
        Ok(match command {
            1001 => RequestMessage::Handshake(epee_encoding::from_bytes(buf)?),
            1002 => RequestMessage::TimedSync(epee_encoding::from_bytes(buf)?),
            1003 => RequestMessage::Ping,
            1007 => RequestMessage::SupportFlags,
            _ => {
                return Err(epee_encoding::Error::Value(
                    "Failed to decode message, unknown command",
                ))
            }
        })
    }

    fn build(&self, builder: &mut BucketBuilder) -> Result<(), epee_encoding::Error> {
        match self {
            RequestMessage::Handshake(hs) => {
                builder.set_command(1001);
                builder.set_body(epee_encoding::to_bytes(hs)?);
            }
            RequestMessage::TimedSync(ts) => {
                builder.set_command(1002);
                builder.set_body(epee_encoding::to_bytes(ts)?);
            }
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
    fn decode(buf: &[u8], command: u32) -> Result<Self, epee_encoding::Error> {
        Ok(match command {
            1001 => ResponseMessage::Handshake(epee_encoding::from_bytes(buf)?),
            1002 => ResponseMessage::TimedSync(epee_encoding::from_bytes(buf)?),
            1003 => ResponseMessage::Ping(epee_encoding::from_bytes(buf)?),
            1007 => ResponseMessage::SupportFlags(epee_encoding::from_bytes(buf)?),
            _ => {
                return Err(epee_encoding::Error::Value(
                    "Failed to decode message, unknown command",
                ))
            }
        })
    }

    fn build(&self, builder: &mut BucketBuilder) -> Result<(), epee_encoding::Error> {
        match self {
            ResponseMessage::Handshake(hs) => {
                builder.set_command(1001);
                builder.set_body(epee_encoding::to_bytes(hs)?);
            }
            ResponseMessage::TimedSync(ts) => {
                builder.set_command(1002);
                builder.set_body(epee_encoding::to_bytes(ts)?);
            }
            ResponseMessage::Ping(pg) => {
                builder.set_command(1003);
                builder.set_body(epee_encoding::to_bytes(pg)?);
            }
            ResponseMessage::SupportFlags(sf) => {
                builder.set_command(1007);
                builder.set_body(epee_encoding::to_bytes(sf)?);
            }
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
            MessageType::Request => Message::Request(
                RequestMessage::decode(body, command)
                    .map_err(|e| BucketError::BodyDecodingError(Box::new(e)))?,
            ),
            MessageType::Response => Message::Response(
                ResponseMessage::decode(body, command)
                    .map_err(|e| BucketError::BodyDecodingError(Box::new(e)))?,
            ),
            MessageType::Notification => Message::Protocol(
                ProtocolMessage::decode(body, command)
                    .map_err(|e| BucketError::BodyDecodingError(Box::new(e)))?,
            ),
        })
    }

    fn encode(&self, builder: &mut BucketBuilder) -> Result<(), BucketError> {
        match self {
            Message::Protocol(pro) => {
                builder.set_message_type(MessageType::Notification);
                builder.set_return_code(0);
                pro.build(builder)
                    .map_err(|e| BucketError::BodyDecodingError(Box::new(e)))?;
            }
            Message::Request(req) => {
                builder.set_message_type(MessageType::Request);
                builder.set_return_code(0);
                req.build(builder)
                    .map_err(|e| BucketError::BodyDecodingError(Box::new(e)))?;
            }
            Message::Response(res) => {
                builder.set_message_type(MessageType::Response);
                builder.set_return_code(1);
                res.build(builder)
                    .map_err(|e| BucketError::BodyDecodingError(Box::new(e)))?;
            }
        }
        Ok(())
    }
}
