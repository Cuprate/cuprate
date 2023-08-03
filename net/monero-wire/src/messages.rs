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

use levin_cuprate::{
    BucketBuilder, BucketError, LevinBody, LevinCommand as LevinCommandTrait, MessageType,
};

pub mod admin;
pub mod common;
pub mod protocol;

pub use admin::{
    HandshakeRequest, HandshakeResponse, PingResponse, SupportFlagsResponse, TimedSyncRequest,
    TimedSyncResponse,
};
pub use common::{BasicNodeData, CoreSyncData, PeerID, PeerListEntryBase};
pub use protocol::{
    ChainRequest, ChainResponse, FluffyMissingTransactionsRequest, GetObjectsRequest,
    GetObjectsResponse, GetTxPoolCompliment, NewBlock, NewFluffyBlock, NewTransactions,
};

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
            2010 => LevinCommand::NewBlock,

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
    fn decode(buf: &[u8], command: LevinCommand) -> Result<Self, epee_encoding::Error> {
        Ok(match command {
            LevinCommand::NewBlock => ProtocolMessage::NewBlock(epee_encoding::from_bytes(buf)?),
            LevinCommand::NewTransactions => {
                ProtocolMessage::NewTransactions(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::GetObjectsRequest => {
                ProtocolMessage::GetObjectsRequest(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::GetObjectsResponse => {
                ProtocolMessage::GetObjectsResponse(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::ChainRequest => {
                ProtocolMessage::ChainRequest(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::ChainResponse => {
                ProtocolMessage::ChainEntryResponse(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::NewFluffyBlock => {
                ProtocolMessage::NewFluffyBlock(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::FluffyMissingTxsRequest => {
                ProtocolMessage::FluffyMissingTransactionsRequest(epee_encoding::from_bytes(buf)?)
            }
            LevinCommand::GetTxPoolCompliment => {
                ProtocolMessage::GetTxPoolCompliment(epee_encoding::from_bytes(buf)?)
            }
            _ => {
                return Err(epee_encoding::Error::Value(
                    "Failed to decode message, unknown command",
                ))
            }
        })
    }

    fn encode(&self) -> Result<Vec<u8>, epee_encoding::Error> {
        match self {
            ProtocolMessage::NewBlock(nb) => epee_encoding::to_bytes(nb),
            ProtocolMessage::NewTransactions(nt) => epee_encoding::to_bytes(nt),
            ProtocolMessage::GetObjectsRequest(gt) => epee_encoding::to_bytes(gt),
            ProtocolMessage::GetObjectsResponse(ge) => epee_encoding::to_bytes(ge),
            ProtocolMessage::ChainRequest(ct) => epee_encoding::to_bytes(ct),
            ProtocolMessage::ChainEntryResponse(ce) => epee_encoding::to_bytes(ce),
            ProtocolMessage::NewFluffyBlock(fb) => epee_encoding::to_bytes(fb),
            ProtocolMessage::FluffyMissingTransactionsRequest(ft) => epee_encoding::to_bytes(ft),
            ProtocolMessage::GetTxPoolCompliment(tp) => epee_encoding::to_bytes(tp),
        }
    }

    fn command(&self) -> LevinCommand {
        match self {
            ProtocolMessage::NewBlock(_) => LevinCommand::NewBlock,
            ProtocolMessage::NewTransactions(_) => LevinCommand::NewTransactions,
            ProtocolMessage::GetObjectsRequest(_) => LevinCommand::GetObjectsRequest,
            ProtocolMessage::GetObjectsResponse(_) => LevinCommand::GetObjectsResponse,
            ProtocolMessage::ChainRequest(_) => LevinCommand::ChainRequest,
            ProtocolMessage::ChainEntryResponse(_) => LevinCommand::ChainResponse,
            ProtocolMessage::NewFluffyBlock(_) => LevinCommand::NewFluffyBlock,
            ProtocolMessage::FluffyMissingTransactionsRequest(_) => {
                LevinCommand::FluffyMissingTxsRequest
            }
            ProtocolMessage::GetTxPoolCompliment(_) => LevinCommand::GetTxPoolCompliment,
        }
    }

    fn build(&self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), epee_encoding::Error> {
        builder.set_command(self.command());
        builder.set_body(self.encode()?);
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
    fn decode(buf: &[u8], command: LevinCommand) -> Result<Self, epee_encoding::Error> {
        Ok(match command {
            LevinCommand::Handshake => RequestMessage::Handshake(epee_encoding::from_bytes(buf)?),
            LevinCommand::TimedSync => RequestMessage::TimedSync(epee_encoding::from_bytes(buf)?),
            LevinCommand::Ping => RequestMessage::Ping,
            LevinCommand::SupportFlags => RequestMessage::SupportFlags,
            _ => {
                return Err(epee_encoding::Error::Value(
                    "Failed to decode message, unknown command",
                ))
            }
        })
    }

    fn command(&self) -> LevinCommand {
        match self {
            RequestMessage::Handshake(_) => LevinCommand::Handshake,
            RequestMessage::TimedSync(_) => LevinCommand::TimedSync,
            RequestMessage::Ping => LevinCommand::Ping,
            RequestMessage::SupportFlags => LevinCommand::SupportFlags,
        }
    }

    fn encode(&self) -> Result<Vec<u8>, epee_encoding::Error> {
        match self {
            RequestMessage::Handshake(x) => epee_encoding::to_bytes(x),
            RequestMessage::TimedSync(x) => epee_encoding::to_bytes(x),
            RequestMessage::Ping => Ok(vec![]),
            RequestMessage::SupportFlags => Ok(vec![]),
        }
    }

    fn build(&self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), epee_encoding::Error> {
        builder.set_command(self.command());
        builder.set_body(self.encode()?);
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
    fn decode(buf: &[u8], command: LevinCommand) -> Result<Self, epee_encoding::Error> {
        Ok(match command {
            LevinCommand::Handshake => ResponseMessage::Handshake(epee_encoding::from_bytes(buf)?),
            LevinCommand::TimedSync => ResponseMessage::TimedSync(epee_encoding::from_bytes(buf)?),
            LevinCommand::Ping => ResponseMessage::Ping(epee_encoding::from_bytes(buf)?),
            LevinCommand::SupportFlags => {
                ResponseMessage::SupportFlags(epee_encoding::from_bytes(buf)?)
            }
            _ => {
                return Err(epee_encoding::Error::Value(
                    "Failed to decode message, unknown command",
                ))
            }
        })
    }

    fn command(&self) -> LevinCommand {
        match self {
            ResponseMessage::Handshake(_) => LevinCommand::Handshake,
            ResponseMessage::TimedSync(_) => LevinCommand::TimedSync,
            ResponseMessage::Ping(_) => LevinCommand::Ping,
            ResponseMessage::SupportFlags(_) => LevinCommand::SupportFlags,
        }
    }

    fn encode(&self) -> Result<Vec<u8>, epee_encoding::Error> {
        match self {
            ResponseMessage::Handshake(x) => epee_encoding::to_bytes(x),
            ResponseMessage::TimedSync(x) => epee_encoding::to_bytes(x),
            ResponseMessage::Ping(x) => epee_encoding::to_bytes(x),
            ResponseMessage::SupportFlags(x) => epee_encoding::to_bytes(x),
        }
    }

    fn build(&self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), epee_encoding::Error> {
        builder.set_command(self.command());
        builder.set_body(self.encode()?);
        Ok(())
    }
}

pub enum Message {
    Request(RequestMessage),
    Response(ResponseMessage),
    Protocol(ProtocolMessage),
}

impl LevinBody for Message {
    type Command = LevinCommand;

    fn decode_message(
        body: &[u8],
        typ: MessageType,
        command: LevinCommand,
    ) -> Result<Self, BucketError> {
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

    fn encode(&self, builder: &mut BucketBuilder<LevinCommand>) -> Result<(), BucketError> {
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
