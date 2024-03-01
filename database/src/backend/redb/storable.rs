//! `cuprate_database::Storable` <-> `redb` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{any::Any, borrow::Cow, marker::PhantomData};

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
    fn compare(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
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
