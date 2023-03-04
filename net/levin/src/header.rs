//! This module provides a struct BucketHead for the header of a levin protocol
//! message.

use std::io::Read;

use super::{BucketError, LEVIN_SIGNATURE, PROTOCOL_VERSION};

use byteorder::{LittleEndian, ReadBytesExt};

/// The Flags for the levin header
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Flags(u32);

pub(crate) const REQUEST: Flags = Flags(0b0000_0001);
pub(crate) const RESPONSE: Flags = Flags(0b0000_0010);
const START_FRAGMENT: Flags = Flags(0b0000_0100);
const END_FRAGMENT: Flags = Flags(0b0000_1000);
const DUMMY: Flags = Flags(0b0000_1100); // both start and end fragment set

impl Flags {
    fn contains_flag(&self, rhs: Self) -> bool {
        self & &rhs == rhs
    }

    /// Converts the inner flags to little endidan bytes
    pub fn to_le_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }

    /// Checks if the flags have the `REQUEST` flag set and
    /// does not have the `RESPONSE` flag set, this does
    /// not check for other flags
    pub fn is_request(&self) -> bool {
        self.contains_flag(REQUEST) && !self.contains_flag(RESPONSE)
    }

    /// Checks if the flags have the `RESPONSE` flag set and
    /// does not have the `REQUEST` flag set, this does
    /// not check for other flags
    pub fn is_response(&self) -> bool {
        self.contains_flag(RESPONSE) && !self.contains_flag(REQUEST)
    }

    /// Checks if the flags have the `START_FRAGMENT`and the
    /// `END_FRAGMENT` flags set, this does
    /// not check for other flags
    pub fn is_dummy(&self) -> bool {
        self.contains_flag(DUMMY)
    }

    /// Checks if the flags have the `START_FRAGMENT` flag
    /// set and does not have the `END_FRAGMENT` flag set, this
    /// does not check for other flags
    pub fn is_start_fragment(&self) -> bool {
        self.contains_flag(START_FRAGMENT) && !self.is_dummy()
    }

    /// Checks if the flags have the `END_FRAGMENT` flag
    /// set and does not have the `START_FRAGMENT` flag set, this
    /// does not check for other flags
    pub fn is_end_fragment(&self) -> bool {
        self.contains_flag(END_FRAGMENT) && !self.is_dummy()
    }

    /// Sets the `REQUEST` flag
    pub fn set_flag_request(&mut self) {
        *self |= REQUEST
    }

    /// Sets the `RESPONSE` flag
    pub fn set_flag_response(&mut self) {
        *self |= RESPONSE
    }

    /// Sets the `START_FRAGMENT` flag
    pub fn set_flag_start_fragment(&mut self) {
        *self |= START_FRAGMENT
    }

    /// Sets the `END_FRAGMENT` flag
    pub fn set_flag_end_fragment(&mut self) {
        *self |= END_FRAGMENT
    }

    /// Sets the `START_FRAGMENT` and `END_FRAGMENT` flag
    pub fn set_flag_dummy(&mut self) {
        self.set_flag_start_fragment();
        self.set_flag_end_fragment();
    }
}

impl From<u32> for Flags {
    fn from(value: u32) -> Self {
        Flags(value)
    }
}

impl core::ops::BitAnd for &Flags {
    type Output = Flags;
    fn bitand(self, rhs: Self) -> Self::Output {
        Flags(self.0 & rhs.0)
    }
}

impl core::ops::BitOr for &Flags {
    type Output = Flags;
    fn bitor(self, rhs: Self) -> Self::Output {
        Flags(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for Flags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
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
            flags: Flags::from(r.read_u32::<LittleEndian>()?),
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
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&self.protocol_version.to_le_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Flags;

    #[test]
    fn set_flags() {
        macro_rules! set_and_check {
            ($set:ident, $check:ident) => {
                let mut flag = Flags::default();
                flag.$set();
                assert!(flag.$check());
            };
        }
        set_and_check!(set_flag_request, is_request);
        set_and_check!(set_flag_response, is_response);
        set_and_check!(set_flag_start_fragment, is_start_fragment);
        set_and_check!(set_flag_end_fragment, is_end_fragment);
        set_and_check!(set_flag_dummy, is_dummy);
    }
}
