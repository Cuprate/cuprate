//! (De)serialization for table keys & values.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    fmt::Debug,
    io::{Read, Write},
    sync::Arc,
};

use bytemuck::{AnyBitPattern, NoUninit};

//---------------------------------------------------------------------------------------------------- Storable
/// Storable types in the database.
///
/// All keys and values in the database must be able
/// to be (de)serialized into/from raw bytes (`[u8]`).
///
/// This trait represents types that can be **perfectly**
/// casted/represented as raw bytes.
///
/// # `bytemuck`
/// Any type that implements `bytemuck`'s [`NoUninit`] + [`AnyBitPattern`]
/// (and [Debug]) will automatically implement [`Storable`].
///
/// This includes:
/// - Most primitive types
/// - All types in [`tables`](crate::tables)
/// - Slices, e.g, `[T] where T: Storable`
///
/// ```rust
/// # use cuprate_database::*;
/// let number: u64 = 0;
///
/// // Into bytes.
/// let into = Storable::as_bytes(&number);
/// assert_eq!(into, &[0; 8]);
///
/// // From bytes.
/// let from: &u64 = Storable::from_bytes(&into);
/// assert_eq!(from, &number);
/// ```
///
/// # Invariants
/// No function in this trait is expected to panic.
///
/// The byte conversions must execute flawlessly.
pub trait Storable: Debug {
    /// Is this type fixed width in byte length?
    ///
    /// I.e., when converting `Self` to bytes, is it
    /// represented with a fixed length array of bytes?
    ///
    /// # `Some`
    /// This should be `Some(usize)` on types like:
    /// - `u8`
    /// - `u64`
    /// - `i32`
    ///
    /// where the byte length is known.
    ///
    /// # `None`
    /// This should be `None` on any variable-length type like:
    /// - `str`
    /// - `[u8]`
    /// - `Vec<u8>`
    const BYTE_LENGTH: Option<usize>;

    /// Return `self` in byte form.
    fn as_bytes(&self) -> &[u8];

    /// Create [`Self`] from bytes.
    fn from_bytes(bytes: &[u8]) -> &Self;
}

//---------------------------------------------------------------------------------------------------- Impl
impl<T: NoUninit + AnyBitPattern + Debug> Storable for T {
    const BYTE_LENGTH: Option<usize> = Some(std::mem::size_of::<T>());

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::from_bytes(bytes)
    }
}

impl<T: NoUninit + AnyBitPattern + Debug> Storable for [T] {
    const BYTE_LENGTH: Option<usize> = None;

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::must_cast_slice(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::must_cast_slice(bytes)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;

    // TODO: copy all `pod.rs` tests here.
}
