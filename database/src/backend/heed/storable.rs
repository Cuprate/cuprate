//! `cuprate_database::Storable` <-> `heed` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{borrow::Cow, fmt::Debug, marker::PhantomData};

use heed::{types::Bytes, BoxedError, BytesDecode, BytesEncode, Database};

use crate::{storable::Storable, storable_slice::StorableSlice};

//---------------------------------------------------------------------------------------------------- StorableHeed
/// The glue struct that implements `heed`'s (de)serialization
/// traits on any type that implements `cuprate_database::Storable`.
///
/// Never actually gets constructed, just used for trait bound translations.
pub(super) struct StorableHeed<T>(PhantomData<T>)
where
    T: Storable + ?Sized;

//---------------------------------------------------------------------------------------------------- BytesDecode
impl<'a, T> BytesDecode<'a> for StorableHeed<T>
where
    T: Storable + 'static,
{
    type DItem = T;

    #[inline]
    /// This function is infallible (will always return `Ok`).
    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        Ok(T::from_bytes(bytes))
    }
}

//---------------------------------------------------------------------------------------------------- BytesEncode
impl<'a, T> BytesEncode<'a> for StorableHeed<T>
where
    T: Storable + ?Sized + 'a,
{
    type EItem = T;

    #[inline]
    /// This function is infallible (will always return `Ok`).
    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        Ok(Cow::Borrowed(item.as_bytes()))
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    // Each `#[test]` function has a `test()` to:
    // - log
    // - simplify trait bounds
    // - make sure the right function is being called

    #[test]
    /// Assert `BytesEncode::bytes_encode` is accurate.
    fn bytes_encode() {
        fn test<T>(t: &T, expected: &[u8])
        where
            T: Storable + ?Sized,
        {
            println!("t: {t:?}, expected: {expected:?}");
            assert_eq!(
                <StorableHeed::<T> as BytesEncode>::bytes_encode(t).unwrap(),
                expected
            );
        }

        test::<()>(&(), &[]);
        test::<u8>(&0, &[0]);
        test::<u16>(&1, &[1, 0]);
        test::<u32>(&2, &[2, 0, 0, 0]);
        test::<u64>(&3, &[3, 0, 0, 0, 0, 0, 0, 0]);
        test::<i8>(&-1, &[255]);
        test::<i16>(&-2, &[254, 255]);
        test::<i32>(&-3, &[253, 255, 255, 255]);
        test::<i64>(&-4, &[252, 255, 255, 255, 255, 255, 255, 255]);
        test::<StorableSlice<u8>>(&StorableSlice::Slice([1, 2].as_slice()), &[1, 2]);
        test::<[u8; 0]>(&[], &[]);
        test::<[u8; 1]>(&[255], &[255]);
        test::<[u8; 2]>(&[111, 0], &[111, 0]);
        test::<[u8; 3]>(&[1, 0, 1], &[1, 0, 1]);
    }

    #[test]
    /// Assert `BytesDecode::bytes_decode` is accurate.
    fn bytes_decode() {
        fn test<T>(bytes: &[u8], expected: &T)
        where
            T: Storable + PartialEq + ToOwned + Debug,
            T::Owned: Debug,
        {
            println!("bytes: {bytes:?}, expected: {expected:?}");
            assert_eq!(
                &<StorableHeed::<T> as BytesDecode>::bytes_decode(bytes).unwrap(),
                expected
            );
        }

        test::<()>([].as_slice(), &());
        test::<u8>([0].as_slice(), &0);
        test::<u16>([1, 0].as_slice(), &1);
        test::<u32>([2, 0, 0, 0].as_slice(), &2);
        test::<u64>([3, 0, 0, 0, 0, 0, 0, 0].as_slice(), &3);
        test::<i8>([255].as_slice(), &-1);
        test::<i16>([254, 255].as_slice(), &-2);
        test::<i32>([253, 255, 255, 255].as_slice(), &-3);
        test::<i64>([252, 255, 255, 255, 255, 255, 255, 255].as_slice(), &-4);
        test::<StorableSlice<u8>>(&[1, 2], &StorableSlice::Slice([1, 2].as_slice()));
        test::<[u8; 0]>([].as_slice(), &[]);
        test::<[u8; 1]>([255].as_slice(), &[255]);
        test::<[u8; 2]>([111, 0].as_slice(), &[111, 0]);
        test::<[u8; 3]>([1, 0, 1].as_slice(), &[1, 0, 1]);
    }
}
