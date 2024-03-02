//! `cuprate_database::Storable` <-> `redb` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{any::Any, borrow::Cow, cmp::Ordering, marker::PhantomData};

use redb::{RedbKey, RedbValue, TypeName};

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- StorableRedb
/// The glue struct that implements `redb`'s (de)serialization
/// traits on any type that implements `cuprate_database::Key`.
///
/// Never actually gets constructed, just used for trait bound translations.
#[derive(Debug)]
pub(super) struct StorableRedb<T: Storable + ?Sized>(PhantomData<T>);

//---------------------------------------------------------------------------------------------------- RedbKey
// If `Key` is also implemented, this can act as a `RedbKey`.
impl<T: Key + ?Sized> RedbKey for StorableRedb<T> {
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        <T as Key>::compare(left, right)
    }
}

//---------------------------------------------------------------------------------------------------- RedbValue
impl<T: Storable + ?Sized> RedbValue for StorableRedb<T> {
    type SelfType<'a> = &'a T where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    #[inline]
    fn fixed_width() -> Option<usize> {
        <T as Storable>::BYTE_LENGTH
    }

    #[inline]
    fn from_bytes<'a>(data: &'a [u8]) -> &'a T
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
mod test {
    use super::*;

    #[test]
    /// Assert `RedbKey::compare` works for `StorableRedb`.
    fn compare() {
        let left = 0;
        let right = 1;
        assert_eq!(
            <StorableRedb::<u64> as RedbKey>::compare(
                Storable::as_bytes(&left),
                Storable::as_bytes(&right)
            ),
            Ordering::Less
        );

        let left = [1, 1];
        let right = [1, 0];
        assert_eq!(
            <StorableRedb::<[u8; 2]> as RedbKey>::compare(
                Storable::as_bytes(&left),
                Storable::as_bytes(&right)
            ),
            Ordering::Greater
        );

        let left = [1, 2, 3];
        let right = [1, 2, 3];
        assert_eq!(
            <StorableRedb::<[i32; 2]> as RedbKey>::compare(
                Storable::as_bytes(&left),
                Storable::as_bytes(&right)
            ),
            Ordering::Equal
        );
    }

    #[test]
    /// Assert `RedbKey::fixed_width` is accurate.
    fn fixed_width() {
        assert_eq!(StorableRedb::<()>::fixed_width(), Some(0));
        assert_eq!(StorableRedb::<u8>::fixed_width(), Some(1));
        assert_eq!(StorableRedb::<u16>::fixed_width(), Some(2));
        assert_eq!(StorableRedb::<u32>::fixed_width(), Some(4));
        assert_eq!(StorableRedb::<u64>::fixed_width(), Some(8));
        assert_eq!(StorableRedb::<i8>::fixed_width(), Some(1));
        assert_eq!(StorableRedb::<i16>::fixed_width(), Some(2));
        assert_eq!(StorableRedb::<i32>::fixed_width(), Some(4));
        assert_eq!(StorableRedb::<i64>::fixed_width(), Some(8));
        assert_eq!(StorableRedb::<[u8]>::fixed_width(), None);
        assert_eq!(StorableRedb::<[u8; 0]>::fixed_width(), Some(0));
        assert_eq!(StorableRedb::<[u8; 1]>::fixed_width(), Some(1));
        assert_eq!(StorableRedb::<[u8; 2]>::fixed_width(), Some(2));
        assert_eq!(StorableRedb::<[u8; 3]>::fixed_width(), Some(3));
    }

    #[test]
    /// Assert `RedbKey::as_bytes` is accurate.
    fn as_bytes() {
        assert_eq!(StorableRedb::<()>::as_bytes(&&()), []);
        assert_eq!(StorableRedb::<u8>::as_bytes(&&0), [0]);
        assert_eq!(StorableRedb::<u16>::as_bytes(&&1), [1, 0]);
        assert_eq!(StorableRedb::<u32>::as_bytes(&&2), [2, 0, 0, 0]);
        assert_eq!(StorableRedb::<u64>::as_bytes(&&3), [3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(StorableRedb::<i8>::as_bytes(&&-1), [255]);
        assert_eq!(StorableRedb::<i16>::as_bytes(&&-2), [254, 255]);
        assert_eq!(StorableRedb::<i32>::as_bytes(&&-3), [253, 255, 255, 255]);
        assert_eq!(
            StorableRedb::<i64>::as_bytes(&&-4),
            [252, 255, 255, 255, 255, 255, 255, 255]
        );
        assert_eq!(StorableRedb::<[u8]>::as_bytes(&[1, 2].as_slice()), [1, 2]);
        assert_eq!(StorableRedb::<[u8; 0]>::as_bytes(&&[]), []);
        assert_eq!(StorableRedb::<[u8; 1]>::as_bytes(&&[255]), [255]);
        assert_eq!(StorableRedb::<[u8; 2]>::as_bytes(&&[111, 0]), [111, 0]);
        assert_eq!(StorableRedb::<[u8; 3]>::as_bytes(&&[1, 0, 1]), [1, 0, 1]);
    }

    #[test]
    /// Assert `RedbKey::from_bytes` is accurate.
    fn from_bytes() {
        assert_eq!(StorableRedb::<()>::from_bytes([].as_slice()), &());
        assert_eq!(StorableRedb::<u8>::from_bytes([0].as_slice()), &0);
        assert_eq!(StorableRedb::<u16>::from_bytes([1, 0].as_slice()), &1);
        assert_eq!(StorableRedb::<u32>::from_bytes([2, 0, 0, 0].as_slice()), &2);
        assert_eq!(
            StorableRedb::<u64>::from_bytes([3, 0, 0, 0, 0, 0, 0, 0].as_slice()),
            &3
        );
        assert_eq!(StorableRedb::<i8>::from_bytes([255].as_slice()), &-1);
        assert_eq!(StorableRedb::<i16>::from_bytes([254, 255].as_slice()), &-2);
        assert_eq!(
            StorableRedb::<i32>::from_bytes([253, 255, 255, 255].as_slice()),
            &-3
        );
        assert_eq!(
            StorableRedb::<i64>::from_bytes([252, 255, 255, 255, 255, 255, 255, 255].as_slice()),
            &-4
        );
        assert_eq!(StorableRedb::<[u8]>::from_bytes([1, 2].as_slice()), [1, 2]);
        assert_eq!(StorableRedb::<[u8; 0]>::from_bytes([].as_slice()), &[]);
        assert_eq!(
            StorableRedb::<[u8; 1]>::from_bytes([255].as_slice()),
            &[255]
        );
        assert_eq!(
            StorableRedb::<[u8; 2]>::from_bytes([111, 0].as_slice()),
            &[111, 0]
        );
        assert_eq!(
            StorableRedb::<[u8; 3]>::from_bytes([1, 0, 1].as_slice()),
            &[1, 0, 1]
        );
    }

    #[test]
    /// Assert `RedbKey::type_name` returns unique names.
    /// The name itself isn't tested, the invariant is that
    /// they are all unique.
    fn type_name() {
        // Can't use a proper set because `redb::TypeName: !Ord`.
        let set = [
            StorableRedb::<()>::type_name(),
            StorableRedb::<u8>::type_name(),
            StorableRedb::<u16>::type_name(),
            StorableRedb::<u32>::type_name(),
            StorableRedb::<u64>::type_name(),
            StorableRedb::<i8>::type_name(),
            StorableRedb::<i16>::type_name(),
            StorableRedb::<i32>::type_name(),
            StorableRedb::<i64>::type_name(),
            StorableRedb::<[u8]>::type_name(),
            StorableRedb::<[u8; 0]>::type_name(),
            StorableRedb::<[u8; 1]>::type_name(),
            StorableRedb::<[u8; 2]>::type_name(),
            StorableRedb::<[u8; 3]>::type_name(),
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
