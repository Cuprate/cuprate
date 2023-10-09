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
//! header serialization and allows developers to define their own bucket bodies, for a complete
//! monero protocol crate see: monero-wire.
//!
//! ## License
//!
//! This project is licensed under the MIT License.

// Coding conventions
#![forbid(unsafe_code)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(unused_mut)]
//#![deny(missing_docs)]

pub mod codec;
pub mod header;

pub use codec::LevinCodec;
pub use header::BucketHead;

use std::fmt::Debug;

use thiserror::Error;

const PROTOCOL_VERSION: u32 = 1;
const LEVIN_SIGNATURE: u64 = 0x0101010101012101;
const LEVIN_DEFAULT_MAX_PACKET_SIZE: u64 = 100_000_000; // 100MB

/// Possible Errors when working with levin buckets
#[derive(Error, Debug)]
pub enum BucketError {
    /// Invalid header flags
    #[error("Invalid header flags: {0}")]
    InvalidHeaderFlags(&'static str),
    /// Levin bucket exceeded max size
    #[error("Levin bucket exceeded max size")]
    BucketExceededMaxSize,
    /// Invalid Fragmented Message
    #[error("Levin fragmented message was invalid: {0}")]
    InvalidFragmentedMessage(&'static str),
    /// Error decoding the body
    #[error("Error decoding bucket body: {0}")]
    BodyDecodingError(Box<dyn std::error::Error>),
    /// The levin command is unknown
    #[error("The levin command is unknown")]
    UnknownCommand,
    /// I/O error
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}

/// A levin Bucket
#[derive(Debug)]
pub struct Bucket {
    /// The bucket header
    pub header: BucketHead,
    /// The bucket body
    pub body: Vec<u8>,
}

/// An enum representing if the message is a request, response or notification.
#[derive(Debug, Eq, PartialEq)]
pub enum MessageType {
    /// Request
    Request,
    /// Response
    Response,
    /// Notification
    Notification,
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
    pub fn from_flags_and_have_to_return(
        flags: header::Flags,
        have_to_return: bool,
    ) -> Result<Self, BucketError> {
        if flags.request && have_to_return {
            Ok(MessageType::Request)
        } else if flags.request {
            Ok(MessageType::Notification)
        } else if flags.response && !have_to_return {
            Ok(MessageType::Response)
        } else {
            Err(BucketError::InvalidHeaderFlags(
                "Unable to assign a message type to this bucket",
            ))
        }
    }

    pub fn as_flags(&self) -> header::Flags {
        match self {
            MessageType::Request | MessageType::Notification => header::Flags {
                request: true,
                ..Default::default()
            },
            MessageType::Response => header::Flags {
                response: true,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug)]
pub struct BucketBuilder {
    signature: Option<u64>,
    ty: Option<MessageType>,
    command: Option<u32>,
    return_code: Option<i32>,
    protocol_version: Option<u32>,
    body: Option<Vec<u8>>,
}

impl Default for BucketBuilder {
    fn default() -> Self {
        Self {
            signature: Some(LEVIN_SIGNATURE),
            ty: None,
            command: None,
            return_code: None,
            protocol_version: Some(PROTOCOL_VERSION),
            body: None,
        }
    }
}

impl BucketBuilder {
    pub fn set_signature(&mut self, sig: u64) {
        self.signature = Some(sig)
    }

    pub fn set_message_type(&mut self, ty: MessageType) {
        self.ty = Some(ty)
    }

    pub fn set_command(&mut self, command: u32) {
        self.command = Some(command)
    }

    pub fn set_return_code(&mut self, code: i32) {
        self.return_code = Some(code)
    }

    pub fn set_protocol_version(&mut self, version: u32) {
        self.protocol_version = Some(version)
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = Some(body)
    }

    pub fn finish(self) -> Bucket {
        let body = self.body.unwrap();
        let ty = self.ty.unwrap();
        Bucket {
            header: BucketHead {
                signature: self.signature.unwrap(),
                size: body.len().try_into().unwrap(),
                have_to_return_data: ty.have_to_return_data(),
                command: self.command.unwrap(),
                return_code: self.return_code.unwrap(),
                flags: ty.as_flags(),
                protocol_version: self.protocol_version.unwrap(),
            },
            body,
        }
    }
}

/// A levin body
pub trait LevinBody: Sized {
    /// Decodes the message from the data in the header
    fn decode_message(body: &[u8], typ: MessageType, command: u32) -> Result<Self, BucketError>;

    /// Encodes the message
    fn encode(&self, builder: &mut BucketBuilder) -> Result<(), BucketError>;
}
