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

pub mod admin;
pub mod common;
pub mod protocol;

pub use common::{BasicNodeData, CoreSyncData, PeerID, PeerListEntryBase};
pub use admin::{Handshake, TimedSync, Ping, SupportFlags};
pub use protocol::{
    NewBlock, NewTransactions, GetObjectsRequest, GetObjectsResponse, ChainRequest, ChainResponse, NewFluffyBlock,
    FluffyMissingTransactionsRequest, GetTxPoolCompliment,
};

use levin::{MessageType, BucketError};

fn zero_val<T: From<u8>>() -> T {
    T::from(0_u8)
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[sealed::sealed]
pub trait NetworkMessage: Sized {
    type EncodingError: std::fmt::Debug;
    fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError>;
    fn encode(&self) -> Result<Vec<u8>, Self::EncodingError>;
}

#[sealed::sealed]
pub trait AdminMessage {
    const ID: u32;
    type Request: NetworkMessage;
    type Response: NetworkMessage;
}

#[sealed::sealed]
pub trait ProtocolMessage {
    const ID: u32;
    type Notification: NetworkMessage;
}

macro_rules! p2p_command {
    ($($message:ident),+) => {
        pub enum P2pCommand {
            $($message,)+
        }

        pub struct P2pCommandFromU32Err;
        impl TryFrom<u32> for P2pCommand {
            type Error = P2pCommandFromU32Err;
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $($message::ID => Ok(P2pCommand::$message),)+
                    _ => Err(P2pCommandFromU32Err)
                }
            }
        }
        impl From<P2pCommand> for u32 {
            fn from(val: P2pCommand) -> Self {
                match val {
                    $(P2pCommand::$message => $message::ID,)+
                }
            }
        }
    };
}

macro_rules! levin_body {
    (
        Admin:
            $($admin_mes:ident),+
        Protocol:
            $($protocol_mes:ident),+
        ) => {

        pub enum MessageRequest {
            $($admin_mes(<$admin_mes as AdminMessage>::Request),)+
        }

        impl MessageRequest {
            pub fn decode(buf: &[u8], command: u32) -> Result<Self, BucketError> {
                match command {
                    $($admin_mes::ID => Ok(
                        MessageRequest::$admin_mes(<$admin_mes as AdminMessage>::Request::decode(buf)
                        .map_err(|e| BucketError::FailedToDecodeBucketBody(e.to_string()))?)),)+ 
                    _ => Err(BucketError::UnsupportedP2pCommand(command))
                }
            }

            pub fn encode(&self) -> Result<(u32, Vec<u8>), BucketError> {
                match self {
                    $(MessageRequest::$admin_mes(mes) => Ok(($admin_mes::ID, mes.encode()
                        .map_err(|e| BucketError::FailedToEncodeBucketBody(e.to_string()))?)),)+
                }
            }
        }


        pub enum MessageResponse {
            $($admin_mes(<$admin_mes as AdminMessage>::Response),)+
        }

        impl MessageResponse {
            pub fn decode(buf: &[u8], command: u32) -> Result<Self, BucketError> {
                match command {
                    $($admin_mes::ID => Ok(
                        MessageResponse::$admin_mes(<$admin_mes as AdminMessage>::Response::decode(buf)
                        .map_err(|e| BucketError::FailedToDecodeBucketBody(e.to_string()))?)),)+
                    _ => Err(BucketError::UnsupportedP2pCommand(command)) 
                }
            }

            pub fn encode(&self) -> Result<(u32, Vec<u8>), BucketError> {
                match self {
                    $(MessageResponse::$admin_mes(mes) => Ok(($admin_mes::ID, mes.encode()
                        .map_err(|e| BucketError::FailedToEncodeBucketBody(e.to_string()))?)),)+
                }
            }
        }

        pub enum MessageNotification {
            $($protocol_mes(<$protocol_mes as ProtocolMessage>::Notification),)+
        }

        impl MessageNotification {
            pub fn decode(buf: &[u8], command: u32) -> Result<Self, BucketError> {
                match command {
                    $($protocol_mes::ID => Ok(
                        MessageNotification::$protocol_mes(<$protocol_mes as ProtocolMessage>::Notification::decode(buf)
                        .map_err(|e| BucketError::FailedToDecodeBucketBody(e.to_string()))?)),)+ 
                    _ => Err(BucketError::UnsupportedP2pCommand(command))
                }
            }

            pub fn encode(&self) -> Result<(u32, Vec<u8>), BucketError> {
                match self {
                    $(MessageNotification::$protocol_mes(mes) => Ok(($protocol_mes::ID, mes.encode()
                        .map_err(|e| BucketError::FailedToEncodeBucketBody(e.to_string()))?)),)+
                }
            }
        }

        pub enum Message {
            Request(MessageRequest),
            Response(MessageResponse),
            Notification(MessageNotification)
        }

        impl levin::LevinBody for Message {
            fn decode_message(buf: &[u8], typ: MessageType, command: u32) -> Result<Self, BucketError> {
                Ok(match typ {
                    MessageType::Response => Message::Response(MessageResponse::decode(buf, command)?),
                    MessageType::Request => Message::Request(MessageRequest::decode(buf, command)?),
                    MessageType::Notification => Message::Notification(MessageNotification::decode(buf, command)?),
                })
            }

            fn encode(&self) -> Result<(i32, u32, MessageType, Vec<u8>), BucketError> {
                match self {
                    Message::Response(mes) => {
                        let (command, bytes)= mes.encode()?;
                        Ok((1, command, MessageType::Response, bytes))
                    },
                    Message::Request(mes) => {
                        let (command, bytes)= mes.encode()?;
                        Ok((0, command, MessageType::Request, bytes))
                    },
                    Message::Notification(mes) => {
                        let (command, bytes)= mes.encode()?;
                        Ok((0, command, MessageType::Notification, bytes))
                    },
                }
            }


        }
        
    };
}

p2p_command!(
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
    FluffyMissingTransactionsRequest,
    GetTxPoolCompliment
);

levin_body!(
    Admin:
        Handshake,
        TimedSync,
        Ping,
        SupportFlags
    Protocol:
        NewBlock,
        NewTransactions,
        GetObjectsRequest,
        GetObjectsResponse,
        ChainRequest,
        ChainResponse,
        NewFluffyBlock,
        FluffyMissingTransactionsRequest,
        GetTxPoolCompliment
);
