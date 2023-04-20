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

use bytes::Bytes;
use levin::BucketError;
use levin::MessageType;

use crate::P2pCommand;

fn zero_val<T: From<u8>>() -> T {
    T::from(0_u8)
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

/// A message request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRequest {
    /// Handshake
    Handshake(admin::HandshakeRequest),
    /// TimedSync
    TimedSync(admin::TimedSyncRequest),
    /// Ping
    Ping(admin::PingRequest),
    /// SupportFlags
    SupportFlags(admin::SupportFlagsRequest),
}

/// A message response
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageResponse {
    /// Handshake
    Handshake(admin::HandshakeResponse),
    /// TimedSync
    TimedSync(admin::TimedSyncResponse),
    /// Ping
    Ping(admin::PingResponse),
    /// SupportFlags
    SupportFlags(admin::SupportFlagsResponse),
}

/// A messages notification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageNotification {
    /// NewBlock
    NewBlock(protocol::NewBlock),
    /// NewTransactions
    NewTransactions(protocol::NewTransactions),
    /// RequestGetObject
    RequestGetObject(protocol::GetObjectsRequest),
    /// ResponseGetObject
    ResponseGetObject(protocol::GetObjectsResponse),
    /// RequestChain
    RequestChain(protocol::ChainRequest),
    /// ResponseChainEntry
    ResponseChainEntry(protocol::ChainResponse),
    /// NewFluffyBlock
    NewFluffyBlock(protocol::NewFluffyBlock),
    /// RequestFluffyMissingTx
    RequestFluffyMissingTx(protocol::FluffyMissingTransactionsRequest),
    /// GetTxPoolComplement
    GetTxPoolComplement(protocol::TxPoolCompliment),
}

/// A Monero Message (levin body)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Request
    Request(MessageRequest),
    /// Response
    Response(MessageResponse),
    /// Notification
    Notification(Box<MessageNotification>), // check benefits/ drawbacks of doing this, im just boxing it for now to satisfy clippy
}

fn epee_encode_error_to_levin(err: epee_serde::Error) -> BucketError {
    BucketError::FailedToEncodeBucketBody(err.to_string())
}

fn encode_message<T: serde::ser::Serialize>(message: &T) -> Result<Vec<u8>, BucketError> {
    epee_serde::to_bytes(message).map_err(epee_encode_error_to_levin)
}

fn epee_decode_error_to_levin(err: epee_serde::Error) -> BucketError {
    BucketError::FailedToDecodeBucketBody(err.to_string())
}

fn decode_message<T: serde::de::DeserializeOwned>(buf: &[u8]) -> Result<T, BucketError> {
    epee_serde::from_bytes(buf).map_err(epee_decode_error_to_levin)
}

impl levin::LevinBody for Message {
    fn decode_message(
        buf: &[u8],
        typ: MessageType,
        have_to_return: bool,
        command: u32,
    ) -> Result<Self, BucketError> {
        let command = P2pCommand::try_from(command)?;

        Ok(match typ {
            MessageType::Response => Message::Response(match command {
                P2pCommand::Handshake => MessageResponse::Handshake(decode_message(buf)?),
                P2pCommand::TimedSync => MessageResponse::TimedSync(decode_message(buf)?),
                P2pCommand::Ping => MessageResponse::Ping(decode_message(buf)?),
                P2pCommand::SupportFlags => MessageResponse::SupportFlags(decode_message(buf)?),
                _ => {
                    return Err(levin::BucketError::FailedToDecodeBucketBody(
                        "Invalid header flag/command/have_to_return combination".to_string(),
                    ))
                }
            }),

            MessageType::Request if have_to_return => Message::Request(match command {
                P2pCommand::Handshake => MessageRequest::Handshake(decode_message(buf)?),
                P2pCommand::TimedSync => MessageRequest::TimedSync(decode_message(buf)?),
                P2pCommand::Ping => MessageRequest::Ping(admin::PingRequest),
                P2pCommand::SupportFlags => {
                    MessageRequest::SupportFlags(admin::SupportFlagsRequest)
                }
                _ => {
                    return Err(levin::BucketError::FailedToDecodeBucketBody(
                        "Invalid header flag/command/have_to_return combination".to_string(),
                    ))
                }
            }),

            MessageType::Request if !have_to_return => {
                Message::Notification(Box::new(match command {
                    P2pCommand::NewBlock => MessageNotification::NewBlock(decode_message(buf)?),
                    P2pCommand::NewTransactions => {
                        MessageNotification::NewTransactions(decode_message(buf)?)
                    }
                    P2pCommand::RequestGetObject => {
                        MessageNotification::RequestGetObject(decode_message(buf)?)
                    }
                    P2pCommand::ResponseGetObject => {
                        MessageNotification::ResponseGetObject(decode_message(buf)?)
                    }
                    P2pCommand::RequestChain => {
                        MessageNotification::RequestChain(decode_message(buf)?)
                    }
                    P2pCommand::ResponseChainEntry => {
                        MessageNotification::ResponseChainEntry(decode_message(buf)?)
                    }
                    P2pCommand::NewFluffyBlock => {
                        MessageNotification::NewFluffyBlock(decode_message(buf)?)
                    }
                    P2pCommand::RequestFluffyMissingTx => {
                        MessageNotification::RequestFluffyMissingTx(decode_message(buf)?)
                    }
                    P2pCommand::GetTxPoolComplement => {
                        MessageNotification::GetTxPoolComplement(decode_message(buf)?)
                    }
                    _ => {
                        return Err(levin::BucketError::FailedToDecodeBucketBody(
                            "Invalid header flag/command/have_to_return combination".to_string(),
                        ))
                    }
                }))
            }
            _ => unreachable!("All typs are handleded"),
        })
    }

    fn encode(&self) -> Result<(i32, u32, bool, MessageType, Bytes), BucketError> {
        let return_code;
        let command;
        let have_to_return_data;
        let flag;
        let bytes;

        match self {
            Message::Request(req) => {
                return_code = 0;
                have_to_return_data = true;
                flag = MessageType::Request;
                match req {
                    MessageRequest::Handshake(handshake) => {
                        command = P2pCommand::Handshake;
                        bytes = encode_message(handshake)?;
                    }
                    MessageRequest::TimedSync(timedsync) => {
                        command = P2pCommand::TimedSync;
                        bytes = encode_message(timedsync)?;
                    }
                    MessageRequest::Ping(_) => {
                        command = P2pCommand::Ping;
                        bytes = Vec::new();
                    }
                    MessageRequest::SupportFlags(_) => {
                        command = P2pCommand::SupportFlags;
                        bytes = Vec::new();
                    }
                }
            }
            Message::Response(res) => {
                return_code = 1;
                have_to_return_data = false;
                flag = MessageType::Response;
                match res {
                    MessageResponse::Handshake(handshake) => {
                        command = P2pCommand::Handshake;
                        bytes = encode_message(handshake)?;
                    }
                    MessageResponse::TimedSync(timed_sync) => {
                        command = P2pCommand::TimedSync;
                        bytes = encode_message(timed_sync)?;
                    }
                    MessageResponse::Ping(ping) => {
                        command = P2pCommand::Ping;
                        bytes = encode_message(ping)?;
                    }
                    MessageResponse::SupportFlags(support_flags) => {
                        command = P2pCommand::SupportFlags;
                        bytes = encode_message(support_flags)?;
                    }
                }
            }
            Message::Notification(noti) => {
                return_code = 0;
                have_to_return_data = false;
                flag = MessageType::Response;
                match noti.as_ref() {
                    MessageNotification::NewBlock(new_block) => {
                        command = P2pCommand::NewBlock;
                        bytes = encode_message(new_block)?;
                    }
                    MessageNotification::NewTransactions(new_txs) => {
                        command = P2pCommand::NewTransactions;
                        bytes = encode_message(new_txs)?;
                    }
                    MessageNotification::RequestGetObject(obj) => {
                        command = P2pCommand::RequestGetObject;
                        bytes = encode_message(obj)?;
                    }
                    MessageNotification::ResponseGetObject(obj) => {
                        command = P2pCommand::ResponseGetObject;
                        bytes = encode_message(obj)?;
                    }
                    MessageNotification::RequestChain(chain) => {
                        command = P2pCommand::RequestChain;
                        bytes = encode_message(chain)?;
                    }
                    MessageNotification::ResponseChainEntry(chain_entry) => {
                        command = P2pCommand::ResponseChainEntry;
                        bytes = encode_message(chain_entry)?;
                    }
                    MessageNotification::NewFluffyBlock(fluffy_block) => {
                        command = P2pCommand::NewFluffyBlock;
                        bytes = encode_message(fluffy_block)?;
                    }
                    MessageNotification::RequestFluffyMissingTx(txs) => {
                        command = P2pCommand::RequestFluffyMissingTx;
                        bytes = encode_message(txs)?;
                    }
                    MessageNotification::GetTxPoolComplement(txpool) => {
                        command = P2pCommand::GetTxPoolComplement;
                        bytes = encode_message(txpool)?;
                    }
                }
            }
        }
        return Ok((
            return_code,
            command.into(),
            have_to_return_data,
            flag,
            bytes.into(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::Message;
    use levin::{LevinBody, MessageType};

    #[test]
    fn decode_handshake_request() {
        let buf = [
            1, 17, 1, 1, 1, 1, 2, 1, 1, 12, 9, 110, 111, 100, 101, 95, 100, 97, 116, 97, 12, 24, 7,
            109, 121, 95, 112, 111, 114, 116, 6, 168, 70, 0, 0, 10, 110, 101, 116, 119, 111, 114,
            107, 95, 105, 100, 10, 64, 18, 48, 241, 113, 97, 4, 65, 97, 23, 49, 0, 130, 22, 161,
            161, 16, 7, 112, 101, 101, 114, 95, 105, 100, 5, 153, 5, 227, 61, 188, 214, 159, 10,
            13, 115, 117, 112, 112, 111, 114, 116, 95, 102, 108, 97, 103, 115, 6, 1, 0, 0, 0, 8,
            114, 112, 99, 95, 112, 111, 114, 116, 7, 0, 0, 20, 114, 112, 99, 95, 99, 114, 101, 100,
            105, 116, 115, 95, 112, 101, 114, 95, 104, 97, 115, 104, 6, 0, 0, 0, 0, 12, 112, 97,
            121, 108, 111, 97, 100, 95, 100, 97, 116, 97, 12, 24, 21, 99, 117, 109, 117, 108, 97,
            116, 105, 118, 101, 95, 100, 105, 102, 102, 105, 99, 117, 108, 116, 121, 5, 59, 90,
            163, 153, 0, 0, 0, 0, 27, 99, 117, 109, 117, 108, 97, 116, 105, 118, 101, 95, 100, 105,
            102, 102, 105, 99, 117, 108, 116, 121, 95, 116, 111, 112, 54, 52, 5, 0, 0, 0, 0, 0, 0,
            0, 0, 14, 99, 117, 114, 114, 101, 110, 116, 95, 104, 101, 105, 103, 104, 116, 5, 190,
            50, 0, 0, 0, 0, 0, 0, 12, 112, 114, 117, 110, 105, 110, 103, 95, 115, 101, 101, 100, 6,
            0, 0, 0, 0, 6, 116, 111, 112, 95, 105, 100, 10, 128, 230, 40, 186, 45, 79, 79, 224,
            164, 117, 133, 84, 130, 185, 94, 4, 1, 57, 126, 74, 145, 238, 238, 122, 44, 214, 85,
            129, 237, 230, 14, 67, 218, 11, 116, 111, 112, 95, 118, 101, 114, 115, 105, 111, 110,
            8, 1, 18, 108, 111, 99, 97, 108, 95, 112, 101, 101, 114, 108, 105, 115, 116, 95, 110,
            101, 119, 140, 4, 24, 3, 97, 100, 114, 12, 8, 4, 116, 121, 112, 101, 8, 1, 4, 97, 100,
            100, 114, 12, 8, 4, 109, 95, 105, 112, 6, 225, 219, 21, 0, 6, 109, 95, 112, 111, 114,
            116, 7, 0, 0, 2, 105, 100, 5, 0, 0, 0, 0, 0, 0, 0, 0, 9, 108, 97, 115, 116, 95, 115,
            101, 101, 110, 1, 0, 0, 0, 0, 0, 0, 0, 0, 12, 112, 114, 117, 110, 105, 110, 103, 95,
            115, 101, 101, 100, 6, 0, 0, 0, 0, 8, 114, 112, 99, 95, 112, 111, 114, 116, 7, 0, 0,
            20, 114, 112, 99, 95, 99, 114, 101, 100, 105, 116, 115, 95, 112, 101, 114, 95, 104, 97,
            115, 104, 6, 0, 0, 0, 0,
        ];

        let message = Message::decode_message(&buf, MessageType::Request, true, 1001);
        println!("{:?}", message);
    }
}
