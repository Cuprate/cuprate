//! This module provides a struct BucketHead for the header of a levin protocol
//! message. It also defines the Flags bitflags enum which contains the possible
//! flag values.

use std::io::Read;

use super::{BucketError, LEVIN_SIGNATURE, PROTOCOL_VERSION};

use bitflags::bitflags;
use byteorder::{LittleEndian, ReadBytesExt};

bitflags! {
    /// Flags for the levin header
    pub struct Flags: u32 {
        /// Used for requests or notifications
        const REQUEST = 1;
        /// Used for Responses
        const RESPONSE = 2;
        /// Used in Fragmented messages
        const START_FRAGMENT = 4;
        /// Used in Fragmented messages
        const END_FRAGMENT = 8;
        /// Used in Tor/i2p connections
        const DUMMY = Self::START_FRAGMENT.bits | Self::END_FRAGMENT.bits;
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
    pub fn from_bytes<R: Read + ?Sized>(r: &mut R) -> Result<BucketHead, BucketError> {
        let header = BucketHead {
            signature: r.read_u64::<LittleEndian>()?,
            size: r.read_u64::<LittleEndian>()?,
            have_to_return_data: r.read_u8()? != 0,
            command: r.read_u32::<LittleEndian>()?,
            return_code: r.read_i32::<LittleEndian>()?,
            // this is incorrect an will not work for fragmented messages
            flags: Flags::from_bits(r.read_u32::<LittleEndian>()?)
                .ok_or(BucketError::UnknownFlags)?,
            protocol_version: r.read_u32::<LittleEndian>()?,
        };

        Ok(header)
    }

    /// Serializes the header 
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(BucketHead::SIZE);
        out.extend_from_slice(&self.signature.to_le_bytes());
        out.extend_from_slice(&self.size.to_le_bytes());
        out.push(if self.have_to_return_data { 1 } else { 0 });
        out.extend_from_slice(&self.command.to_le_bytes());
        out.extend_from_slice(&self.return_code.to_le_bytes());
        out.extend_from_slice(&self.flags.bits.to_le_bytes());
        out.extend_from_slice(&self.protocol_version.to_le_bytes());
        out
    }
}
