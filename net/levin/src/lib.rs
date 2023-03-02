//! # Rust Levin
//!
//! A crate for working with the Levin protocol in Rust.
//!
//! The Levin protocol is a network protocol used in the Monero cryptocurrency. It is used for
//! peer-to-peer communication between nodes. This crate provides a Rustimplementation of the Levin
//! header serilisation and allows developers to define thier own bucket bodies so this is not a
//! complete monero netowrking crate for that see: ############
//! ## License
//!
//! This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.

pub mod bucket_sink;
pub mod bucket_stream;
pub mod header;

use std::fmt::Debug;

pub use bucket_stream::BucketStream;
use bytes::Bytes;
pub use header::BucketHead;

use thiserror::Error;

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
        [self.header.to_bytes().into(), self.body.clone()].concat().into() // this is probably inefficient I will fix later 
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound,
}
