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
pub mod message_sink;
pub mod bucket_stream;
pub mod message_stream;
pub mod header;

pub use header::BucketHead;

use std::fmt::Debug;

use thiserror::Error;
use bytes::Bytes;

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

pub const PROTOCOL_VERSION: u32 = 1;
pub const LEVIN_SIGNATURE: u64 = 0x0101010101012101;

#[derive(Debug)]
pub struct Bucket {
    header: BucketHead,
    body: Bytes,
}

impl Bucket {
    fn to_bytes(&self) -> Bytes {
        let mut buf =self.header.to_bytes();
        buf.extend(self.body.iter());
        buf.into()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound,
}

#[derive(Debug)]
pub enum BucketType {
    Request,
    Response
}

impl Into<header::Flags> for BucketType {
    fn into(self) -> header::Flags {
        match self {
            BucketType::Request => header::Flags::REQUEST,
            BucketType::Response => header::Flags::RESPONSE
        }
    }
}

pub trait LevinBody: Sized {
    fn decode_message(
        buf: &[u8],
        flags: BucketType,
        have_to_return: bool,
        command: u32,
    ) -> Result<Self, BucketError>;

    /// Encodes the message 
    /// 
    /// returns: 
    ///     return_code: i32,
    ///     command: u32,
    ///     have_to_return: bool,
    ///     flag: Flags - must only be Request or Response
    ///     bytes: Bytes
    fn encode(&self) -> Result<(i32, u32, bool, BucketType, Bytes), BucketError>;
}
