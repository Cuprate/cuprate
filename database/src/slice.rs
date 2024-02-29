//! Generic slice of `T` that is [`Storable`].

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{AnyBitPattern, NoUninit, TransparentWrapper};

#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::storable::Storable;

//---------------------------------------------------------------------------------------------------- Table
/// Generic slice of `T` that is [`Storable`].
///
/// TODO
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, TransparentWrapper)]
#[repr(transparent)]
pub struct Slice<T>(
    /* TODO: is there a reason this should private? */ pub [T],
);

//---------------------------------------------------------------------------------------------------- Storable
impl<T: NoUninit + AnyBitPattern> Storable for Slice<T> {
    #[inline]
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        bytemuck::must_cast_slice(&self.0)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        let t_slice: &[T] = bytemuck::must_cast_slice(bytes);
        TransparentWrapper::wrap_ref(t_slice)
    }
}

//---------------------------------------------------------------------------------------------------- Traits
impl<T> std::ops::Deref for Slice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<[T]> for Slice<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<'a, T: NoUninit + AnyBitPattern> From<&'a [T]> for &'a Slice<T> {
    #[inline]
    fn from(value: &'a [T]) -> Self {
        TransparentWrapper::wrap_ref(value)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
