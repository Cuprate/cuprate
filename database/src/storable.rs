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

use bytemuck::{AnyBitPattern, CheckedBitPattern, NoUninit};

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
    /// - `Vec<u8>`
    /// - etc.
    fn as_bytes(&self) -> impl AsRef<[u8]>;

    /// Create [`Self`] from bytes.
    ///
    /// # Panics
    /// In the case `bytes` is invalid, this should panic.
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

/// This macro exists because some of our types are _NOT_
/// `AnyBitPattern`, i.e. their bits must be checked for
/// validity before casting.
///
/// Notably, any type that contains a `bool` must be checked.
/// Since any `AnyBitPattern` type implements `CheckedBitPattern`,
/// we cannot do another blanket implementation like this:
///
/// ```rust,ignore
/// impl<T: NoUninit + CheckedBitPattern> Storable for T { /* ... */ }
/// ```
///
/// We could also just use the above and check everytime,
/// but that is a waste on types that can be flawlessly
/// casted from bytes, so instead, the few types that
/// must be checked will be implemented here.
macro_rules! impl_storable_checked_bit_pattern {
    ($(
        // The type to implement `Storable` on.
        // It must also implement `NoUnit + CheckedBitPattern`.
        $t:ty
    ),* $(,)?) => {
        $(
            impl Storable for $t {
                fn as_bytes(&self) -> impl AsRef<[u8]> {
                    bytemuck::bytes_of(self)
                }

                fn from_bytes(bytes: &[u8]) -> &Self {
                    bytemuck::checked::from_bytes(bytes)
                }
            }
        )*
    };
}
impl_storable_checked_bit_pattern! {
    crate::types::TestType,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
