use bytes::{Buf, BufMut};

use crate::error::{Error, Result};

const SIZE_OF_SIZE_MARKER: u32 = 2;
const FITS_IN_ONE_BYTE: u64 = 2_u64.pow(8 - SIZE_OF_SIZE_MARKER) - 1;
const FITS_IN_TWO_BYTES: u64 = 2_u64.pow(16 - SIZE_OF_SIZE_MARKER) - 1;
const FITS_IN_FOUR_BYTES: u64 = 2_u64.pow(32 - SIZE_OF_SIZE_MARKER) - 1;

/// Read an epee variable sized number from `r`.
///
/// ```rust
/// use cuprate_epee_encoding::read_varint;
///
/// assert_eq!(read_varint::<_, u64>(&mut [252].as_slice()).unwrap(), 63);
/// assert_eq!(read_varint::<_, u64>(&mut [1, 1].as_slice()).unwrap(), 64);
/// assert_eq!(read_varint::<_, u64>(&mut [253, 255].as_slice()).unwrap(), 16_383);
/// assert_eq!(read_varint::<_, u64>(&mut [2, 0, 1, 0].as_slice()).unwrap(), 16_384);
/// assert_eq!(read_varint::<_, u64>(&mut [254, 255, 255, 255].as_slice()).unwrap(), 1_073_741_823);
/// assert_eq!(read_varint::<_, u64>(&mut [3, 0, 0, 0, 1, 0, 0, 0].as_slice()).unwrap(), 1_073_741_824);
/// ```
pub fn read_varint<B: Buf, T: TryFrom<u64>>(r: &mut B) -> Result<T> {
    if !r.has_remaining() {
        return Err(Error::IO("Not enough bytes to build VarInt"));
    }

    let vi_start = r.get_u8();
    let len = 1 << (vi_start & 0b11);

    if r.remaining() < len - 1 {
        return Err(Error::IO("Not enough bytes to build VarInt"));
    }

    let mut vi = u64::from(vi_start >> 2);
    for i in 1..len {
        vi |= u64::from(r.get_u8()) << (((i - 1) * 8) + 6);
    }

    vi.try_into().map_err(|_| Error::IO("VarInt is too big"))
}

/// Write an epee variable sized number into `w`.
///
/// ```rust
/// use cuprate_epee_encoding::write_varint;
///
/// let mut buf = vec![];
///
/// for (number, expected_bytes) in [
///     (63, [252].as_slice()),
///     (64, [1, 1].as_slice()),
///     (16_383, [253, 255].as_slice()),
///     (16_384, [2, 0, 1, 0].as_slice()),
///     (1_073_741_823, [254, 255, 255, 255].as_slice()),
///     (1_073_741_824, [3, 0, 0, 0, 1, 0, 0, 0].as_slice()),
/// ] {
///     buf.clear();
///     write_varint(number, &mut buf);
///     assert_eq!(buf.as_slice(), expected_bytes);
/// }
/// ```
pub fn write_varint<B: BufMut, T: TryInto<u64>>(number: T, w: &mut B) -> Result<()> {
    let number = number
        .try_into()
        .map_err(|_| "Tried to write a varint bigger than 64-bits")
        .unwrap();

    let size_marker = match number {
        0..=FITS_IN_ONE_BYTE => 0,
        64..=FITS_IN_TWO_BYTES => 1,
        16384..=FITS_IN_FOUR_BYTES => 2,
        _ => 3,
    };

    if w.remaining_mut() < 1 << size_marker {
        return Err(Error::IO("Not enough capacity to write VarInt"));
    }

    let number = (number << 2) | size_marker;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Although `as` is unsafe we just checked the length."
    )]
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
        assert_eq!(read_varint::<_, u64>(&mut varint).unwrap(), val);
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
