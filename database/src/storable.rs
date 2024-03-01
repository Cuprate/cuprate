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
    /// Return `self` in byte form.
    ///
    /// The returned bytes can be any form of array,
    /// - `[u8]`
    /// - `[u8; N]`
    /// - `Vec<u8>`
    /// - etc.
    fn as_bytes(&self) -> &[u8];

    /// Create [`Self`] from bytes.
    fn from_bytes(bytes: &[u8]) -> &Self;

    /// TODO
    fn fixed_width() -> Option<usize>;
}

//---------------------------------------------------------------------------------------------------- Impl
impl<T: NoUninit + AnyBitPattern + Debug> Storable for T {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::from_bytes(bytes)
    }

    #[inline]
    fn fixed_width() -> Option<usize> {
        Some(std::mem::size_of::<T>())
    }
}

impl<T: NoUninit + AnyBitPattern + Debug> Storable for [T] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::must_cast_slice(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::must_cast_slice(bytes)
    }

    #[inline]
    fn fixed_width() -> Option<usize> {
        None
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;

    // TODO: copy all `pod.rs` tests here.
}
