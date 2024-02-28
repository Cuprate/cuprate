use bytes::{Buf, BufMut};

use crate::error::*;

const SIZE_OF_SIZE_MARKER: u32 = 2;
const FITS_IN_ONE_BYTE: u64 = 2_u64.pow(8 - SIZE_OF_SIZE_MARKER) - 1;
const FITS_IN_TWO_BYTES: u64 = 2_u64.pow(16 - SIZE_OF_SIZE_MARKER) - 1;
const FITS_IN_FOUR_BYTES: u64 = 2_u64.pow(32 - SIZE_OF_SIZE_MARKER) - 1;

pub fn read_varint<B: Buf>(r: &mut B) -> Result<u64> {
    if !r.has_remaining() {
        Err(Error::IO("Not enough bytes to build VarInt"))?
    }

    let vi_start = r.get_u8();
    let len = 1 << (vi_start & 0b11);

    if r.remaining() < len - 1 {
        Err(Error::IO("Not enough bytes to build VarInt"))?
    }

    let mut vi = u64::from(vi_start >> 2);
    for i in 1..len {
        vi |= u64::from(r.get_u8()) << (((i - 1) * 8) + 6);
    }
    Ok(vi)
}

pub fn write_varint<B: BufMut>(number: u64, w: &mut B) -> Result<()> {
    let size_marker = match number {
        0..=FITS_IN_ONE_BYTE => 0,
        64..=FITS_IN_TWO_BYTES => 1,
        16384..=FITS_IN_FOUR_BYTES => 2,
        _ => 3,
    };

    if w.remaining_mut() < 1 << size_marker {
        Err(Error::IO("Not enough capacity to write VarInt"))?;
    }

    let number = (number << 2) | size_marker;

    // Although `as` is unsafe we just checked the length.
    match size_marker {
        0 => w.put_u8(number as u8),
        1 => w.put_u16_le(number as u16),
        2 => w.put_u32_le(number as u32),
        3 => w.put_u64_le(number),
        _ => unreachable!(),
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use alloc::vec::Vec;

    use crate::varint::*;

    fn assert_varint_length(number: u64, len: usize) {
        let mut w = Vec::new();
        write_varint(number, &mut w).unwrap();
        assert_eq!(w.len(), len);
    }

    fn assert_varint_val(mut varint: &[u8], val: u64) {
        assert_eq!(read_varint(&mut varint).unwrap(), val);
    }

    #[test]
    fn varint_write_length() {
        assert_varint_length(FITS_IN_ONE_BYTE, 1);
        assert_varint_length(FITS_IN_ONE_BYTE + 1, 2);
        assert_varint_length(FITS_IN_TWO_BYTES, 2);
        assert_varint_length(FITS_IN_TWO_BYTES + 1, 4);
        assert_varint_length(FITS_IN_FOUR_BYTES, 4);
        assert_varint_length(FITS_IN_FOUR_BYTES + 1, 8);
    }

    #[test]
    fn varint_read() {
        assert_varint_val(&[252], FITS_IN_ONE_BYTE);
        assert_varint_val(&[1, 1], FITS_IN_ONE_BYTE + 1);
        assert_varint_val(&[253, 255], FITS_IN_TWO_BYTES);
        assert_varint_val(&[2, 0, 1, 0], FITS_IN_TWO_BYTES + 1);
        assert_varint_val(&[254, 255, 255, 255], FITS_IN_FOUR_BYTES);
        assert_varint_val(&[3, 0, 0, 0, 1, 0, 0, 0], FITS_IN_FOUR_BYTES + 1);
    }
}
