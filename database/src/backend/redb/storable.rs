//! `cuprate_database::Storable` <-> `redb` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{any::Any, borrow::Cow, cmp::Ordering, fmt::Debug, marker::PhantomData};

use redb::TypeName;

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- StorableRedb
/// The glue structs that implements `redb`'s (de)serialization
/// traits on any type that implements `cuprate_database::Key`.
///
/// Never actually get constructed, just used for trait bound translations.
#[derive(Debug)]
pub(super) struct StorableRedb<T>(PhantomData<T>)
where
    T: Storable;

//---------------------------------------------------------------------------------------------------- redb::Key
// If `Key` is also implemented, this can act as a `redb::Key`.
impl<T> redb::Key for StorableRedb<T>
where
    T: Key + 'static,
{
    #[inline]
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        <T as Key>::compare(left, right)
    }
}

//---------------------------------------------------------------------------------------------------- redb::Value
impl<T> redb::Value for StorableRedb<T>
where
    T: Storable + 'static,
{
    type SelfType<'a> = T where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    #[inline]
    fn fixed_width() -> Option<usize> {
        <T as Storable>::BYTE_LENGTH
    }

    #[inline]
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'static>
    where
        Self: 'a,
    {
        <T as Storable>::from_bytes(data)
    }

    #[inline]
    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> &'a [u8]
    where
        Self: 'a + 'b,
    {
        <T as Storable>::as_bytes(value)
    }

    #[inline]
    fn type_name() -> TypeName {
        TypeName::new(std::any::type_name::<T>())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[allow(clippy::needless_pass_by_value)]
mod test {
    use super::*;
    use crate::{StorableBytes, StorableVec};

    // Each `#[test]` function has a `test()` to:
    // - log
    // - simplify trait bounds
    // - make sure the right function is being called

    #[test]
    /// Assert `redb::Key::compare` works for `StorableRedb`.
    fn compare() {
        fn test<T>(left: T, right: T, expected: Ordering)
        where
            T: Key + 'static,
        {
            println!("left: {left:?}, right: {right:?}, expected: {expected:?}");
            assert_eq!(
                <StorableRedb::<T> as redb::Key>::compare(
                    <StorableRedb::<T> as redb::Value>::as_bytes(&left),
                    <StorableRedb::<T> as redb::Value>::as_bytes(&right)
                ),
                expected
            );
        }

        test::<i64>(-1, 2, Ordering::Greater); // bytes are greater, not the value
        test::<u64>(0, 1, Ordering::Less);
        test::<[u8; 2]>([1, 1], [1, 0], Ordering::Greater);
        test::<[u8; 3]>([1, 2, 3], [1, 2, 3], Ordering::Equal);
    }

    #[test]
    /// Assert `redb::Key::fixed_width` is accurate.
    fn fixed_width() {
        fn test<T>(expected: Option<usize>)
        where
            T: Storable + 'static,
        {
            assert_eq!(<StorableRedb::<T> as redb::Value>::fixed_width(), expected);
        }

        test::<()>(Some(0));
        test::<u8>(Some(1));
        test::<u16>(Some(2));
        test::<u32>(Some(4));
        test::<u64>(Some(8));
        test::<i8>(Some(1));
        test::<i16>(Some(2));
        test::<i32>(Some(4));
        test::<i64>(Some(8));
        test::<StorableVec<u8>>(None);
        test::<StorableBytes>(None);
        test::<[u8; 0]>(Some(0));
        test::<[u8; 1]>(Some(1));
        test::<[u8; 2]>(Some(2));
        test::<[u8; 3]>(Some(3));
    }

    #[test]
    /// Assert `redb::Key::as_bytes` is accurate.
    fn as_bytes() {
        fn test<T>(t: &T, expected: &[u8])
        where
            T: Storable + 'static,
        {
            println!("t: {t:?}, expected: {expected:?}");
            assert_eq!(<StorableRedb::<T> as redb::Value>::as_bytes(t), expected);
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
        test::<StorableVec<u8>>(&StorableVec([1, 2].to_vec()), &[1, 2]);
        test::<StorableBytes>(&StorableBytes(bytes::Bytes::from_static(&[1, 2])), &[1, 2]);
        test::<[u8; 0]>(&[], &[]);
        test::<[u8; 1]>(&[255], &[255]);
        test::<[u8; 2]>(&[111, 0], &[111, 0]);
        test::<[u8; 3]>(&[1, 0, 1], &[1, 0, 1]);
    }

    #[test]
    /// Assert `redb::Key::from_bytes` is accurate.
    fn from_bytes() {
        fn test<T>(bytes: &[u8], expected: &T)
        where
            T: Storable + PartialEq + 'static,
        {
            println!("bytes: {bytes:?}, expected: {expected:?}");
            assert_eq!(
                &<StorableRedb::<T> as redb::Value>::from_bytes(bytes),
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

    #[test]
    /// Assert `redb::Key::type_name` returns unique names.
    /// The name itself isn't tested, the invariant is that
    /// they are all unique.
    fn type_name() {
        // Can't use a proper set because `redb::TypeName: !Ord`.
        let set = [
            <StorableRedb<()> as redb::Value>::type_name(),
            <StorableRedb<u8> as redb::Value>::type_name(),
            <StorableRedb<u16> as redb::Value>::type_name(),
            <StorableRedb<u32> as redb::Value>::type_name(),
            <StorableRedb<u64> as redb::Value>::type_name(),
            <StorableRedb<i8> as redb::Value>::type_name(),
            <StorableRedb<i16> as redb::Value>::type_name(),
            <StorableRedb<i32> as redb::Value>::type_name(),
            <StorableRedb<i64> as redb::Value>::type_name(),
            <StorableRedb<StorableVec<u8>> as redb::Value>::type_name(),
            <StorableRedb<StorableBytes> as redb::Value>::type_name(),
            <StorableRedb<[u8; 0]> as redb::Value>::type_name(),
            <StorableRedb<[u8; 1]> as redb::Value>::type_name(),
            <StorableRedb<[u8; 2]> as redb::Value>::type_name(),
            <StorableRedb<[u8; 3]> as redb::Value>::type_name(),
        ];

        // Check every permutation is unique.
        for (index, i) in set.iter().enumerate() {
            for (index2, j) in set.iter().enumerate() {
                if index != index2 {
                    assert_ne!(i, j);
                }
            }
        }
    }
}
