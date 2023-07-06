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

pub mod codec;
pub mod header;

pub use codec::LevinCodec;
pub use header::BucketHead;

use std::fmt::Debug;

use bytes::Bytes;
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
    pub body: Bytes,
}
