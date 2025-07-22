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

cfg_if::cfg_if! {
    // Used in `tests/`.
    if #[cfg(test)] {
        use futures as _;
        use proptest as _;
        use rand as _;
        use tokio as _;
    }
}

use std::fmt::Debug;

use bytes::{Buf, Bytes};
use thiserror::Error;

use cuprate_helper::cast::usize_to_u64;

pub mod codec;
pub mod header;
pub mod message;

pub use codec::*;
pub use header::BucketHead;
pub use message::LevinMessage;

use header::Flags;

/// The version field for bucket headers.
const MONERO_PROTOCOL_VERSION: u32 = 1;
/// The signature field for bucket headers, will be constant for all peers using the Monero levin
/// protocol.
const MONERO_LEVIN_SIGNATURE: u64 = 0x0101010101012101;
/// Maximum size a bucket can be before a handshake.
const MONERO_MAX_PACKET_SIZE_BEFORE_HANDSHAKE: u64 = 256 * 1000; // 256 KiB
/// Maximum size a bucket can be after a handshake.
const MONERO_MAX_PACKET_SIZE: u64 = 100_000_000; // 100MB

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
    /// The Header did not have the correct signature
    #[error("Levin header had incorrect signature")]
    InvalidHeaderSignature,
    /// Error decoding the body
    #[error("Error decoding bucket body: {0}")]
    BodyDecodingError(Box<dyn std::error::Error + Send + Sync>),
    /// Unknown command ID
    #[error("Unknown command ID")]
    UnknownCommand,
    /// I/O error
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}

/// Levin protocol settings, allows setting custom parameters.
///
/// For Monero use [`Protocol::default()`]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Protocol {
    pub version: u32,
    pub signature: u64,
    pub max_packet_size_before_handshake: u64,
    pub max_packet_size: u64,
}

impl Default for Protocol {
    fn default() -> Self {
        Self {
            version: MONERO_PROTOCOL_VERSION,
            signature: MONERO_LEVIN_SIGNATURE,
            max_packet_size_before_handshake: MONERO_MAX_PACKET_SIZE_BEFORE_HANDSHAKE,
            max_packet_size: MONERO_MAX_PACKET_SIZE,
        }
    }
}

/// A levin Bucket
#[derive(Debug, Clone)]
pub struct Bucket<C> {
    /// The bucket header
    pub header: BucketHead<C>,
    /// The bucket body
    pub body: Bytes,
}

/// An enum representing if the message is a request, response or notification.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    pub const fn have_to_return_data(&self) -> bool {
        match self {
            Self::Request => true,
            Self::Response | Self::Notification => false,
        }
    }

    /// Returns the `MessageType` given the flags and `have_to_return_data` fields
    pub const fn from_flags_and_have_to_return(
        flags: Flags,
        have_to_return: bool,
    ) -> Result<Self, BucketError> {
        Ok(match (flags, have_to_return) {
            (Flags::REQUEST, true) => Self::Request,
            (Flags::REQUEST, false) => Self::Notification,
            (Flags::RESPONSE, false) => Self::Response,
            _ => {
                return Err(BucketError::InvalidHeaderFlags(
                    "Unable to assign a message type to this bucket",
                ))
            }
        })
    }

    pub const fn as_flags(&self) -> Flags {
        match self {
            Self::Request | Self::Notification => Flags::REQUEST,
            Self::Response => Flags::RESPONSE,
        }
    }
}

#[derive(Debug)]
pub struct BucketBuilder<C> {
    signature: Option<u64>,
    ty: Option<MessageType>,
    command: Option<C>,
    return_code: Option<i32>,
    protocol_version: Option<u32>,
    body: Option<Bytes>,
}

impl<C: LevinCommand> BucketBuilder<C> {
    pub const fn new(protocol: &Protocol) -> Self {
        Self {
            signature: Some(protocol.signature),
            ty: None,
            command: None,
            return_code: None,
            protocol_version: Some(protocol.version),
            body: None,
        }
    }

    pub const fn set_signature(&mut self, sig: u64) {
        self.signature = Some(sig);
    }

    pub const fn set_message_type(&mut self, ty: MessageType) {
        self.ty = Some(ty);
    }

    pub fn set_command(&mut self, command: C) {
        self.command = Some(command);
    }

    pub const fn set_return_code(&mut self, code: i32) {
        self.return_code = Some(code);
    }

    pub const fn set_protocol_version(&mut self, version: u32) {
        self.protocol_version = Some(version);
    }

    pub fn set_body(&mut self, body: Bytes) {
        self.body = Some(body);
    }

    pub fn finish(self) -> Bucket<C> {
        let body = self.body.unwrap();
        let ty = self.ty.unwrap();
        Bucket {
            header: BucketHead {
                signature: self.signature.unwrap(),
                size: usize_to_u64(body.len()),
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
    type Command: LevinCommand + Debug;

    /// Decodes the message from the data in the header
    fn decode_message<B: Buf>(
        body: &mut B,
        ty: MessageType,
        command: Self::Command,
    ) -> Result<Self, BucketError>;

    /// Encodes the message
    fn encode(self, builder: &mut BucketBuilder<Self::Command>) -> Result<(), BucketError>;
}

/// The levin commands.
///
/// Implementers should account for all possible u32 values, this means
/// you will probably need some sort of `Unknown` variant.
pub trait LevinCommand: From<u32> + Into<u32> + PartialEq + Clone {
    /// Returns the size limit for this command.
    ///
    /// must be less than [`usize::MAX`]
    fn bucket_size_limit(&self) -> u64;
    /// Returns if this is a handshake
    fn is_handshake(&self) -> bool;
}
