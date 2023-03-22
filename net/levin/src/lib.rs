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

//! # Rust Levin
//!
//! A crate for working with the Levin protocol in Rust.
//!
//! The Levin protocol is a network protocol used in the Monero cryptocurrency. It is used for
//! peer-to-peer communication between nodes. This crate provides a Rust implementation of the Levin
//! header serialization and allows developers to define their own bucket bodies so this is not a
//! complete Monero networking crate.
//!
//! ## License
//!
//! This project is licensed under the MIT License.

// Coding conventions
#![forbid(unsafe_code)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(unused_mut)]
#![deny(missing_docs)]

pub mod bucket_sink;
pub mod bucket_stream;
pub mod header;
pub mod message_sink;
pub mod message_stream;

pub use header::BucketHead;

use std::fmt::Debug;

use bytes::Bytes;
use thiserror::Error;

/// Possible Errors when working with levin buckets
#[derive(Error, Debug)]
pub enum BucketError {
    /// Unsupported p2p command.
    #[error("Unsupported p2p command: {0}")]
    UnsupportedP2pCommand(u32),
    /// Revived header with incorrect signature.
    #[error("Revived header with incorrect signature: {0}")]
    IncorrectSignature(u64),
    /// Header contains unknown flags.
    #[error("Header contains unknown flags")]
    UnknownFlags,
    /// Revived header with unknown protocol version.
    #[error("Revived header with unknown protocol version: {0}")]
    UnknownProtocolVersion(u32),
    /// More bytes needed to parse data.
    #[error("More bytes needed to parse data")]
    NotEnoughBytes,
    /// Failed to decode bucket body.
    #[error("Failed to decode bucket body: {0}")]
    FailedToDecodeBucketBody(String),
    /// Failed to encode bucket body.
    #[error("Failed to encode bucket body: {0}")]
    FailedToEncodeBucketBody(String),
    /// IO Error.
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    /// Peer sent an error response code.
    #[error("Peer sent an error response code: {0}")]
    Error(i32),
}

const PROTOCOL_VERSION: u32 = 1;
const LEVIN_SIGNATURE: u64 = 0x0101010101012101;

/// A levin Bucket
#[derive(Debug)]
pub struct Bucket {
    header: BucketHead,
    body: Bytes,
}

impl Bucket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = self.header.to_bytes();
        buf.extend(self.body.iter());
        buf.into()
    }
}

/// An enum representing if the message is a request or response
#[derive(Debug)]
pub enum MessageType {
    /// Request
    Request,
    /// Response
    Response,
    /// Notification
    Notification
}

impl MessageType {
    /// Returns if the message requires a response
    pub fn have_to_return_data(&self) -> bool {
        match self {
            MessageType::Request => true,
            MessageType::Response | MessageType::Notification => false,
        }
    }

    /// Returns the `MessageType` given the flags and have_to_return_data fields 
    pub fn from_flags_and_have_to_return(flags: header::Flags, have_to_return: bool) -> Result<Self, BucketError> {
        if flags.is_request() && have_to_return {
            Ok(MessageType::Request)
        }
        else if flags.is_request() {
            Ok(MessageType::Notification)
        } else if flags.is_response() && !have_to_return {
            Ok(MessageType::Response)
        } else {
            Err(BucketError::UnknownFlags)
        }
    }
}

impl From<MessageType> for header::Flags {
    fn from(val: MessageType) -> Self {
        match val {
            MessageType::Request | MessageType::Notification => header::REQUEST,
            MessageType::Response => header::RESPONSE,
        }
    }
}

/// A levin body
pub trait LevinBody: Sized {
    /// Decodes the message from the data in the header
    fn decode_message(buf: &[u8], typ: MessageType, command: u32) -> Result<Self, BucketError>;

    /// Encodes the message
    ///
    /// returns:
    ///     return_code: i32,
    ///     command: u32,
    ///     message_type: MessageType
    ///     bytes: Vec<u8>
    fn encode(&self) -> Result<(i32, u32, MessageType, Vec<u8>), BucketError>;
}
