//! A [`Storable`] wrapper type for `[T]`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    char::ToLowercase,
    fmt::Debug,
    io::{Read, Write},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};

use crate::{storable::Storable, to_owned_debug::ToOwnedDebug};

//---------------------------------------------------------------------------------------------------- StorableSlice
/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StorableSlice<'a, T> {
    /// TODO
    Vec(Vec<T>),
    /// TODO
    Slice(&'a [T]),
}

//---------------------------------------------------------------------------------------------------- Storable
impl<T> Storable for StorableSlice<'_, T>
where
    T: Pod + ToOwnedDebug<OwnedDebug = T>,
{
    const ALIGN: usize = std::mem::align_of::<T>();
    const BYTE_LENGTH: Option<usize> = None;

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Vec(vec) => bytemuck::must_cast_slice(vec),
            Self::Slice(slice) => bytemuck::must_cast_slice(slice),
        }
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::Vec(bytemuck::pod_collect_to_vec(bytes))
    }
}

//---------------------------------------------------------------------------------------------------- Traits
impl<T> Deref for StorableSlice<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            Self::Vec(vec) => vec,
            Self::Slice(slice) => slice,
        }
    }
}

impl<T> Borrow<[T]> for StorableSlice<'_, T> {
    fn borrow(&self) -> &[T] {
        match self {
            Self::Vec(vec) => vec.as_slice(),
            Self::Slice(slice) => slice,
        }
    }
}

impl<T> From<Vec<T>> for StorableSlice<'_, T> {
    fn from(value: Vec<T>) -> Self {
        Self::Vec(value)
    }
}

impl<T> From<Box<[T]>> for StorableSlice<'_, T> {
    fn from(value: Box<[T]>) -> Self {
        Self::Vec(value.into_vec())
    }
}

impl<'a, T> From<&'a [T]> for StorableSlice<'_, T> {
    fn from(value: &'a [T]) -> Self {
        Self::Slice(value)
    }
}

impl<'a, const N: usize, T> From<&'a [T; N]> for StorableSlice<'_, T> {
    fn from(value: &'a [T; N]) -> Self {
        Self::Slice(value)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
