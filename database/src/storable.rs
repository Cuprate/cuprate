//! (De)serialization for table keys & values.
//!
//! All keys and values in database tables must be able
//! to be (de)serialized into/from raw bytes ([u8]).

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    io::{Read, Write},
    sync::Arc,
};

use bytemuck::{AnyBitPattern, NoUninit};

//---------------------------------------------------------------------------------------------------- Storable
/// TODO
///
/// Trait representing very simple types that can be
/// (de)serialized into/from bytes.
pub trait Storable {
    /// Return `self` in byte form.
    ///
    /// The returned bytes can be any form of array,
    /// - `[u8]`
    /// - `[u8; N]`
    /// - [`Vec<u8>`]
    ///
    /// ..etc.
    ///
    /// This is used on slice types (`Vec<u8>`, `[u8; N]`, etc) for cheap conversions.
    ///
    /// Integer types ([`u8`], [`f32`], [`i8`], etc) return a fixed-sized array.
    fn as_bytes(&self) -> impl AsRef<[u8]>;

    /// Create [`Self`] from bytes.
    ///
    /// # Panics
    /// This function should be infallible.
    ///
    /// If `bytes` is invalid, this should panic.
    fn from_bytes(bytes: &[u8]) -> &Self;
}

//---------------------------------------------------------------------------------------------------- Impl
impl<T: NoUninit + AnyBitPattern> Storable for T {
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        bytemuck::bytes_of(self)
    }

    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::from_bytes(bytes)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
