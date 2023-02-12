use std::io::Read;

use super::{BucketError, LEVIN_SIGNATURE, PROTOCOL_VERSION};

use bitflags::bitflags;
use byteorder::{LittleEndian, ReadBytesExt};

bitflags! {
    pub struct Flags: u32 {
        const REQUEST = 1;
        const RESPONSE = 2;
        const START_FRAGMENT = 4;
        const END_FRAGMENT = 8;
        const DUMMY = Self::START_FRAGMENT.bits | Self::END_FRAGMENT.bits;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BucketHead {
    pub signature: u64,
    pub size: u64,
    pub have_to_return_data: bool,
    pub command: u32,
    pub return_code: i32,
    pub flags: Flags,
    pub protocol_version: u32,
}

impl BucketHead {
    pub const SIZE: usize = 33;

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

    pub fn from_bytes<R: Read + ?Sized>(r: &mut R) -> Result<BucketHead, BucketError> {
        let header = BucketHead {
            signature: r.read_u64::<LittleEndian>()?,
            size: r.read_u64::<LittleEndian>()?,
            have_to_return_data: r.read_u8()? != 0,
            command: r.read_u32::<LittleEndian>()?,
            return_code: r.read_i32::<LittleEndian>()?,
            flags: Flags::from_bits(r.read_u32::<LittleEndian>()?)
                .ok_or(BucketError::UnknownFlags)?,
            protocol_version: r.read_u32::<LittleEndian>()?,
        };

        if header.signature != LEVIN_SIGNATURE {
            return Err(BucketError::IncorrectSignature(header.signature));
        }

        if header.protocol_version != PROTOCOL_VERSION {
            return Err(BucketError::UnknownProtocolVersion(header.protocol_version));
        }

        if header.return_code < 0 {
            return Err(BucketError::Error(header.return_code));
        }

        Ok(header)
    }

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
