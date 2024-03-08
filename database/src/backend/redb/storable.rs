//! `cuprate_database::Storable` <-> `redb` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{any::Any, borrow::Cow, cmp::Ordering, marker::PhantomData};

use redb::{RedbKey, RedbValue, TypeName};

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- StorableRedbKey
/// The glue structs that implements `redb`'s (de)serialization
/// traits on any type that implements `cuprate_database::Key`.
///
/// Never actually get constructed, just used for trait bound translations.
///
/// HACK: explain why `StorableRedbKey` also exists
/// (prevents Cow, and a bunch of trait bounds at the cost of
/// never being able to retrieve the key, not needed anyway).
#[derive(Debug)]
pub(super) struct StorableRedbKey<T: Storable + ?Sized>(PhantomData<T>);

/// TODO
#[derive(Debug)]
pub(super) struct StorableRedbValue<T: Storable + ?Sized>(PhantomData<T>);

//---------------------------------------------------------------------------------------------------- RedbKey
// If `Key` is also implemented, this can act as a `RedbKey`.
impl<T: Key + ?Sized> RedbKey for StorableRedbKey<T> {
    #[inline]
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        <T as Key>::compare(left, right)
    }
}

//---------------------------------------------------------------------------------------------------- RedbValue
impl<T: Storable + ?Sized> RedbValue for StorableRedbKey<T> {
    type SelfType<'a> = &'a T where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    #[inline]
    fn fixed_width() -> Option<usize> {
        <T as Storable>::BYTE_LENGTH
    }

    #[inline]
    #[allow(clippy::ptr_as_ptr)]
    fn from_bytes<'a>(data: &'a [u8]) -> &'a T
    where
        Self: 'a,
    {
        // Safe on low size keys?
        // This doesn't panic... for now.
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

//---------------------------------------------------------------------------------------------------- RedbValue
impl<T: Storable + Clone + ?Sized> RedbValue for StorableRedbValue<T> {
    type SelfType<'a> = Cow<'a, T> where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    #[inline]
    fn fixed_width() -> Option<usize> {
        <T as Storable>::BYTE_LENGTH
    }

    #[inline]
    #[allow(clippy::ptr_as_ptr)]
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        // Use the bytes directly if possible...
        if std::mem::align_of::<T>() == 1 {
            Cow::Borrowed(<T as Storable>::from_bytes(data))
        // ...else, make sure the bytes are aligned
        // when casting by allocating a new buffer.
        } else {
            <T as Storable>::from_bytes_unaligned(data)
        }
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

    // Each `#[test]` function has a `test()` to:
    // - log
    // - simplify trait bounds
    // - make sure the right function is being called

    #[test]
    /// Assert `RedbKey::compare` works for `StorableRedbKey`.
    fn compare() {
        fn test<T: Key>(left: T, right: T, expected: Ordering) {
            println!("left: {left:?}, right: {right:?}, expected: {expected:?}");
            assert_eq!(
                <StorableRedbKey::<T> as RedbKey>::compare(
                    <StorableRedbKey::<T> as RedbValue>::as_bytes(&&left),
                    <StorableRedbKey::<T> as RedbValue>::as_bytes(&&right)
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
    /// Assert `RedbKey::fixed_width` is accurate.
    fn fixed_width() {
        fn test<T: Storable + ?Sized>(expected: Option<usize>) {
            assert_eq!(<StorableRedbKey::<T> as RedbValue>::fixed_width(), expected);
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
        test::<[u8]>(None);
        test::<[u8; 0]>(Some(0));
        test::<[u8; 1]>(Some(1));
        test::<[u8; 2]>(Some(2));
        test::<[u8; 3]>(Some(3));
    }

    #[test]
    /// Assert `RedbKey::as_bytes` is accurate.
    fn as_bytes() {
        fn test<T: Storable + ?Sized>(t: &T, expected: &[u8]) {
            println!("t: {t:?}, expected: {expected:?}");
            assert_eq!(<StorableRedbKey::<T> as RedbValue>::as_bytes(&t), expected);
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
        test::<[u8]>(&[1, 2], &[1, 2]);
        test::<[u8; 0]>(&[], &[]);
        test::<[u8; 1]>(&[255], &[255]);
        test::<[u8; 2]>(&[111, 0], &[111, 0]);
        test::<[u8; 3]>(&[1, 0, 1], &[1, 0, 1]);
    }

    #[test]
    /// Assert `RedbKey::from_bytes` is accurate.
    fn from_bytes() {
        fn test<T: Storable + ?Sized + PartialEq>(bytes: &[u8], expected: &T) {
            println!("bytes: {bytes:?}, expected: {expected:?}");
            assert_eq!(
                <StorableRedbKey::<T> as RedbValue>::from_bytes(bytes),
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
        test::<[u8]>([1, 2].as_slice(), &[1, 2]);
        test::<[u8; 0]>([].as_slice(), &[]);
        test::<[u8; 1]>([255].as_slice(), &[255]);
        test::<[u8; 2]>([111, 0].as_slice(), &[111, 0]);
        test::<[u8; 3]>([1, 0, 1].as_slice(), &[1, 0, 1]);
    }

    #[test]
    /// Assert `RedbKey::type_name` returns unique names.
    /// The name itself isn't tested, the invariant is that
    /// they are all unique.
    fn type_name() {
        // Can't use a proper set because `redb::TypeName: !Ord`.
        let set = [
            <StorableRedbKey<()> as RedbValue>::type_name(),
            <StorableRedbKey<u8> as RedbValue>::type_name(),
            <StorableRedbKey<u16> as RedbValue>::type_name(),
            <StorableRedbKey<u32> as RedbValue>::type_name(),
            <StorableRedbKey<u64> as RedbValue>::type_name(),
            <StorableRedbKey<i8> as RedbValue>::type_name(),
            <StorableRedbKey<i16> as RedbValue>::type_name(),
            <StorableRedbKey<i32> as RedbValue>::type_name(),
            <StorableRedbKey<i64> as RedbValue>::type_name(),
            <StorableRedbKey<[u8]> as RedbValue>::type_name(),
            <StorableRedbKey<[u8; 0]> as RedbValue>::type_name(),
            <StorableRedbKey<[u8; 1]> as RedbValue>::type_name(),
            <StorableRedbKey<[u8; 2]> as RedbValue>::type_name(),
            <StorableRedbKey<[u8; 3]> as RedbValue>::type_name(),
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
