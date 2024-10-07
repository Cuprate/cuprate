mod into_iter;

use core::ops::Index;
use std::{mem, slice};
use bytes::{BufMut, Bytes, BytesMut};

use crate::FixedByteError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ByteArrayVec<const N: usize>(Bytes);

impl<const N: usize> ByteArrayVec<N> {
    pub fn len(&self) -> usize {
        self.0.len() / N
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn take_bytes(self) -> Bytes {
        self.0
    }

    /// Splits the byte array vec into two at the given index.
    ///
    /// Afterwards self contains elements [0, at), and the returned [`ByteArrayVec`] contains elements [at, len).
    ///
    /// This is an O(1) operation that just increases the reference count and sets a few indices.
    ///
    /// # Panics
    /// Panics if at > len.
    pub fn split_off(&mut self, at: usize) -> Self {
        Self(self.0.split_off(at * N))
    }
}

impl<const N: usize> IntoIterator for ByteArrayVec<N> {
    type Item = [u8; N];
    type IntoIter = into_iter::ByteArrayVecIterator<N>;

    fn into_iter(self) -> Self::IntoIter {
        into_iter::ByteArrayVecIterator(self.0)
    }
}

impl<const N: usize> From<ByteArrayVec<N>> for Vec<[u8; N]> {
    fn from(value: ByteArrayVec<N>) -> Self {
        let vec = value.0.to_vec();

        let mut v = mem::ManuallyDrop::new(vec);

        let p = v.as_mut_ptr().cast::<[u8; N]>();
        let len = v.len() / N;
        let cap = v.capacity() / N;

        unsafe { Vec::from_raw_parts(p, len, cap) }
    }
}

impl<const N: usize> From<Vec<[u8; N]>> for ByteArrayVec<N> {
    fn from(value: Vec<[u8; N]>) -> Self {
        let mut v = mem::ManuallyDrop::new(value);

        let p = v.as_mut_ptr().cast::<u8>();
        let len = v.len() * N;
        let cap = v.capacity() * N;

        let v = unsafe { Vec::from_raw_parts(p, len, cap) };

        ByteArrayVec(Bytes::from(v))
    }
}

impl<const N: usize> TryFrom<Bytes> for ByteArrayVec<N> {
    type Error = FixedByteError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if value.len() % N != 0 {
            return Err(FixedByteError::InvalidLength);
        }

        Ok(ByteArrayVec(value))
    }
}

impl<const N: usize> From<[u8; N]> for ByteArrayVec<N> {
    fn from(value: [u8; N]) -> Self {
        ByteArrayVec(Bytes::copy_from_slice(value.as_slice()))
    }
}

impl<const N: usize, const LEN: usize> From<[[u8; N]; LEN]> for ByteArrayVec<N> {
    fn from(value: [[u8; N]; LEN]) -> Self {
        ByteArrayVec::from(value.to_vec())
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for ByteArrayVec<N> {
    type Error = FixedByteError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() % N != 0 {
            return Err(FixedByteError::InvalidLength);
        }

        Ok(ByteArrayVec(Bytes::from(value)))
    }
}

impl<const N: usize> Index<usize> for ByteArrayVec<N> {
    type Output = [u8; N];

    fn index(&self, index: usize) -> &Self::Output {
        if (index + 1) * N > self.0.len() {
            panic!("Index out of range, idx: {}, length: {}", index, self.len());
        }

        self.0[index * N..(index + 1) * N]
            .as_ref()
            .try_into()
            .unwrap()
    }
}
