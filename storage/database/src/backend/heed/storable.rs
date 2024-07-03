//! `cuprate_database::Storable` <-> `heed` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{borrow::Cow, cmp::Ordering, marker::PhantomData};

use heed::{BoxedError, BytesDecode, BytesEncode};

use crate::{storable::Storable, Key};

//---------------------------------------------------------------------------------------------------- StorableHeed
/// The glue struct that implements `heed`'s (de)serialization
/// traits on any type that implements `cuprate_database::Storable`.
///
/// Never actually gets constructed, just used for trait bound translations.
pub(super) struct StorableHeed<T>(PhantomData<T>)
where
    T: Storable + ?Sized;

//---------------------------------------------------------------------------------------------------- Key
// If `Key` is also implemented, this can act as the comparison function.
impl<T> heed::Comparator for StorableHeed<T>
where
    T: Key,
{
    #[inline]
    fn compare(a: &[u8], b: &[u8]) -> Ordering {
        <T as Key>::KEY_COMPARE.as_compare_fn::<T>()(a, b)
    }
}

//---------------------------------------------------------------------------------------------------- BytesDecode/Encode
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
    use std::fmt::Debug;

    use super::*;
    use crate::{StorableBytes, StorableVec};

    // Each `#[test]` function has a `test()` to:
    // - log
    // - simplify trait bounds
    // - make sure the right function is being called

    #[test]
    /// Assert key comparison behavior is correct.
    fn compare() {
        fn test<T>(left: T, right: T, expected: Ordering)
        where
            T: Key + Ord + 'static,
        {
            println!("left: {left:?}, right: {right:?}, expected: {expected:?}");
            assert_eq!(
                <StorableHeed::<T> as heed::Comparator>::compare(
                    &<StorableHeed::<T> as heed::BytesEncode>::bytes_encode(&left).unwrap(),
                    &<StorableHeed::<T> as heed::BytesEncode>::bytes_encode(&right).unwrap()
                ),
                expected
            );
        }

        // Value comparison
        test::<u8>(0, 255, Ordering::Less);
        test::<u16>(0, 256, Ordering::Less);
        test::<u32>(0, 256, Ordering::Less);
        test::<u64>(0, 256, Ordering::Less);
        test::<u128>(0, 256, Ordering::Less);
        test::<usize>(0, 256, Ordering::Less);
        test::<i8>(-1, 2, Ordering::Less);
        test::<i16>(-1, 2, Ordering::Less);
        test::<i32>(-1, 2, Ordering::Less);
        test::<i64>(-1, 2, Ordering::Less);
        test::<i128>(-1, 2, Ordering::Less);
        test::<isize>(-1, 2, Ordering::Less);

        // Byte comparison
        test::<[u8; 2]>([1, 1], [1, 0], Ordering::Greater);
        test::<[u8; 3]>([1, 2, 3], [1, 2, 3], Ordering::Equal);
    }

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
        test::<StorableVec<u8>>(&StorableVec(vec![1, 2]), &[1, 2]);
        test::<StorableBytes>(&StorableBytes(bytes::Bytes::from_static(&[1, 2])), &[1, 2]);
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
            T: Storable + PartialEq + ToOwned + Debug + 'static,
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
        test::<StorableVec<u8>>(&[1, 2], &StorableVec(vec![1, 2]));
        test::<StorableBytes>(&[1, 2], &StorableBytes(bytes::Bytes::from_static(&[1, 2])));
        test::<[u8; 0]>([].as_slice(), &[]);
        test::<[u8; 1]>([255].as_slice(), &[255]);
        test::<[u8; 2]>([111, 0].as_slice(), &[111, 0]);
        test::<[u8; 3]>([1, 0, 1].as_slice(), &[1, 0, 1]);
    }
}
