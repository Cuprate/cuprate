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

//! # Monero Wire
//!
//! A crate defining Monero network messages and network addresses,
//! built on top of the levin-cuprate crate.
//!
//! ## License
//!
//! This project is licensed under the MIT License.

pub mod network_address;
pub mod p2p;

pub use levin_cuprate::BucketError;
pub use network_address::{NetZone, NetworkAddress};
pub use p2p::*;

// re-export.
pub use levin_cuprate as levin;

pub type MoneroWireCodec = levin_cuprate::codec::LevinMessageCodec<Message>;
