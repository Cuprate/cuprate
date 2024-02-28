use bytes::{Buf, BufMut};

use crate::error::*;

#[inline]
pub fn checked_read_primitive<B: Buf, R: Sized>(
    b: &mut B,
    read: impl Fn(&mut B) -> R,
) -> Result<R> {
    checked_read(b, read, core::mem::size_of::<R>())
}

#[inline]
pub fn checked_read<B: Buf, R>(b: &mut B, read: impl Fn(&mut B) -> R, size: usize) -> Result<R> {
    if b.remaining() < size {
        Err(Error::IO("Not enough bytes in buffer to build object."))?;
    }

    Ok(read(b))
}

#[inline]
pub fn checked_write_primitive<B: BufMut, T: Sized>(
    b: &mut B,
    write: impl Fn(&mut B, T),
    t: T,
) -> Result<()> {
    checked_write(b, write, t, core::mem::size_of::<T>())
}

#[inline]
pub fn checked_write<B: BufMut, T>(
    b: &mut B,
    write: impl Fn(&mut B, T),
    t: T,
    size: usize,
) -> Result<()> {
    if b.remaining_mut() < size {
        Err(Error::IO("Not enough capacity to write object."))?;
    }

    write(b, t);
    Ok(())
}
