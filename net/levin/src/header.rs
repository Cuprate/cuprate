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

//! This module provides a struct BucketHead for the header of a levin protocol
//! message.

use crate::LEVIN_DEFAULT_MAX_PACKET_SIZE;
use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::{BucketError, LEVIN_SIGNATURE, PROTOCOL_VERSION};

const REQUEST: u32 = 0b0000_0001;
const RESPONSE: u32 = 0b0000_0010;
const START_FRAGMENT: u32 = 0b0000_0100;
const END_FRAGMENT: u32 = 0b0000_1000;

/// Levin header flags
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Flags {
    /// Q bit
    pub request: bool,
    /// S bit
    pub response: bool,
    /// B bit
    pub start_fragment: bool,
    /// E bit
    pub end_fragment: bool,
}

impl TryFrom<u32> for Flags {
    type Error = BucketError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let flags = Flags {
            request: value & REQUEST > 0,
            response: value & RESPONSE > 0,
            start_fragment: value & START_FRAGMENT > 0,
            end_fragment: value & END_FRAGMENT > 0,
        };
        if flags.request && flags.response {
            return Err(BucketError::InvalidHeaderFlags(
                "Request and Response bits set",
            ));
        };
        Ok(flags)
    }
}

impl From<Flags> for u32 {
    fn from(value: Flags) -> Self {
        let mut ret = 0;
        if value.request {
            ret |= REQUEST;
        };
        if value.response {
            ret |= RESPONSE;
        };
        if value.start_fragment {
            ret |= START_FRAGMENT;
        };
        if value.end_fragment {
            ret |= END_FRAGMENT;
        };
        ret
    }
}

/// The Header of a Bucket. This contains
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BucketHead {
    /// The network signature, should be `LEVIN_SIGNATURE` for Monero
    pub signature: u64,
    /// The size of the body
    pub size: u64,
    /// If the peer has to send data in the order of requests - some
    /// messages require responses but don't have this set (some notifications)
    pub have_to_return_data: bool,
    /// Command
    pub command: u32,
    /// Return Code - will be 0 for requests and >0 for ok responses otherwise will be
    /// a negative number corresponding to the error
    pub return_code: i32,
    /// The Flags of this header
    pub flags: Flags,
    /// The protocol version, for Monero this is currently 1
    pub protocol_version: u32,
}

impl BucketHead {
    /// The size of the header (in bytes)
    pub const SIZE: usize = 33;

    /// Builds the header in a Monero specific way
    pub fn build(
        payload_size: u64,
        have_to_return_data: bool,
        command: u32,
        flags: Flags,
        return_code: i32,
    ) -> BucketHead {
        BucketHead {
            signature: LEVIN_SIGNATURE,
            size: payload_size,
            have_to_return_data,
            command,
            return_code,
            flags,
            protocol_version: PROTOCOL_VERSION,
        }
    }

    /// Builds the header from bytes, this function does not check any fields should
    /// match the expected ones (signature, protocol_version)
    ///
    /// # Panics
    /// This function will panic if there aren't enough bytes to fill the header.
    /// Currently ['SIZE'](BucketHead::SIZE)
    pub fn from_bytes(buf: &mut BytesMut) -> Result<BucketHead, BucketError> {
        let header = BucketHead {
            signature: buf.get_u64_le(),
            size: buf.get_u64_le(),
            have_to_return_data: buf.get_u8() != 0,
            command: buf.get_u32_le(),
            return_code: buf.get_i32_le(),
            flags: Flags::try_from(buf.get_u32_le())?,
            protocol_version: buf.get_u32_le(),
        };

        if header.size > LEVIN_DEFAULT_MAX_PACKET_SIZE {
            return Err(BucketError::BucketExceededMaxSize);
        }

        Ok(header)
    }

    /// Serializes the header
    pub fn write_bytes(&self, dst: &mut BytesMut) {
        dst.reserve(BucketHead::SIZE);

        dst.put_u64_le(self.signature);
        dst.put_u64_le(self.size);
        dst.put_u8(if self.have_to_return_data { 1 } else { 0 });
        dst.put_u32_le(self.command);
        dst.put_i32_le(self.return_code);
        dst.put_u32_le(self.flags.into());
        dst.put_u32_le(self.protocol_version);
    }
}
