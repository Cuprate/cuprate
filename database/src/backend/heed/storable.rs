//! `cuprate_database::Storable` <-> `heed` serde trait compatibility layer.

//---------------------------------------------------------------------------------------------------- Use
use std::{borrow::Cow, marker::PhantomData};

use heed::{types::Bytes, BoxedError, BytesDecode, BytesEncode, Database};

use crate::storable::Storable;

//---------------------------------------------------------------------------------------------------- StorableHeed
/// The glue struct that implements `heed`'s (de)serialization
/// traits on any type that implements `cuprate_database::Storable`.
///
/// Never actually gets constructed, just used for trait bound translations.
pub(super) struct StorableHeed<T: Storable + ?Sized>(PhantomData<T>);

//---------------------------------------------------------------------------------------------------- BytesDecode
impl<'a, T: Storable + ?Sized + 'a> BytesDecode<'a> for StorableHeed<T> {
    type DItem = &'a T;

    #[inline]
    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, BoxedError> {
        Ok(T::from_bytes(bytes))
    }
}

//---------------------------------------------------------------------------------------------------- BytesEncode
impl<'a, T: Storable + ?Sized + 'a> BytesEncode<'a> for StorableHeed<T> {
    type EItem = T;

    #[inline]
    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, BoxedError> {
        Ok(Cow::Borrowed(item.as_bytes()))
    }
}
