//! `cuprate_database::Storable` <-> `redb` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{any::Any, borrow::Cow, marker::PhantomData};

use redb::{RedbKey, RedbValue, TypeName};

use crate::{key::Key, storable::Storable};

//---------------------------------------------------------------------------------------------------- Types
/// The glue struct that implements `heed`'s (de)serialization
/// traits on any type that implements `cuprate_database::Storable`.
#[derive(Debug)]
pub(super) struct StorableRedbKey<T: Key + ?Sized>(PhantomData<T>);

impl<T: Key + ?Sized> RedbKey for StorableRedbKey<T> {
    fn compare(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
        <T as Key>::compare(left, right)
    }
}

impl<T: Key + ?Sized> RedbValue for StorableRedbKey<T> {
    type SelfType<'a> = &'a T where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    fn fixed_width() -> Option<usize> {
        <T as Storable>::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> &'a T
    where
        Self: 'a,
    {
        <T as Storable>::from_bytes(data)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> &'a [u8]
    where
        Self: 'a + 'b,
    {
        <T as Storable>::as_bytes(value)
    }

    fn type_name() -> TypeName {
        TypeName::new(std::any::type_name::<T>())
    }
}

//---------------------------------------------------------------------------------------------------- Types
/// The glue struct that implements `heed`'s (de)serialization
/// traits on any type that implements `cuprate_database::Storable`.
#[derive(Debug)]
pub(super) struct StorableRedbValue<T: Storable + ?Sized>(PhantomData<T>);

impl<T: Storable + ?Sized> RedbValue for StorableRedbValue<T> {
    type SelfType<'a> = &'a T where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    fn fixed_width() -> Option<usize> {
        <T as Storable>::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> &'a T
    where
        Self: 'a,
    {
        <T as Storable>::from_bytes(data)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> &'a [u8]
    where
        Self: 'a + 'b,
    {
        <T as Storable>::as_bytes(value)
    }

    fn type_name() -> TypeName {
        TypeName::new(std::any::type_name::<T>())
    }
}
