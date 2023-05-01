//! ### MDBX implementation
//! This module contains the implementation of all the database traits for the MDBX storage engine.
//! This include basic transactions methods, cursors and errors conversion.

use crate::{
    database::{Database, transaction::{Transaction, WriteTransaction, Pair, SubPair, FullPair}},
	encoding::{Key, Value, SubKey, Encode, Decode},
    table::{self, DupTable, Table},
    BINCODE_CONFIG, error::DBException,
};
use libmdbx::{
    Cursor, DatabaseFlags, DatabaseKind, Geometry, Mode, PageSize, SyncMode, TableFlags,
    TransactionKind, WriteFlags, RO, RW,
};
use std::ops::Range;

// Constant used in mdbx implementation
const MDBX_DEFAULT_SYNC_MODE: SyncMode = SyncMode::Durable;
const MDBX_MAX_MAP_SIZE: usize = 4 * 1024usize.pow(3); // 4TB
const MDBX_GROWTH_STEP: isize = 100 * 1024isize.pow(2); // 100MB
const MDBX_PAGE_SIZE: Option<PageSize> = None;
const MDBX_GEOMETRY: Geometry<Range<usize>> = Geometry {
    size: Some(0..MDBX_MAX_MAP_SIZE),
    growth_step: Some(MDBX_GROWTH_STEP),
    shrink_threshold: None,
    page_size: MDBX_PAGE_SIZE,
};

/// [`mdbx_return_pair`] is a convenient implementation used at the end of cursors methods to either return the raw pair or decoded pair from database
fn mdbx_return_pair<const B: bool, T: Table>(pair: (<T::Key as Encode>::Output, <T::Value as Encode>::Output)) -> Result<Option<Pair<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		Ok(Some((Key::Raw(pair.0),Value::Raw(pair.1))))
	} else {
		// Extracting then decoding the bytes form the database
		Ok(Some((
			Key::Type(<T::Key as Decode>::decode(&pair.0)?),
			Value::Type(<T::Value as Decode>::decode(&pair.1)?)
		)))
	}
}

/// [`mdbx_return_subpair`] is a convenient implementation used at the end of cursors methods to either return the raw pair or decoded pair from database
fn mdbx_return_subpair<const B: bool, T: DupTable>(subpair: <(T::SubKey, T::Value) as Encode>::Output) -> Result<Option<SubPair<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		Ok(Some(SubPair::Raw(subpair)))
	} else {
		// Extracting then decoding the bytes form the database
		let (subkey,value) = <(T::SubKey, T::Value) as Decode>::decode(&subpair)?;
		Ok(Some(
			SubPair::<T>::Type(
				SubKey::Type(subkey),
				Value::Type(value)
			)
		))
	}
}

/// [`mdbx_return_subpair`] is a convenient implementation used at the end of cursors methods to either return the raw pair or decoded pair from database
fn mdbx_return_fullpair<const B: bool, T: DupTable>(fullpair: (<T::Key as Encode>::Output, <(T::SubKey, T::Value) as Encode>::Output)) -> Result<Option<FullPair<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		Ok(Some((Key::Raw(fullpair.0), SubPair::Raw(fullpair.1))))
	} else {
		// Extracting then decoding the bytes form the database
		let (subkey,value) = <(T::SubKey, T::Value) as Decode>::decode(&fullpair.1)?;
		let key = <T::Key as Decode>::decode(&fullpair.0)?;
		Ok(Some(
			(
				Key::Type(key),
				SubPair::<T>::Type(
					SubKey::Type(subkey),
					Value::Type(value)
				)
			)
		))
	}
}

/// [`mdbx_return_value`] is a convenient implementation used at the end of certain methods to either return the raw value or its decoded type from database
fn mdbx_return_value<const B: bool, T: Table>(value: <T::Value as Encode>::Output) -> Result<Option<Value<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		Ok(Some(Value::Raw(value)))
	} else {
		// Extracting then decoding the bytes form the database
		Ok(Some(
			Value::Type(<T::Value as Decode>::decode(&value)?)
		))
	}
}

// Implementation of the Cursor trait for mdbx's Cursors
impl<'t, T, R> crate::database::transaction::Cursor<'t, T> for Cursor<'t, R>
where
    T: Table,
    R: TransactionKind,
	<T::Key as Encode>::Output: libmdbx::Decodable<'t>,
	<T::Value as Encode>::Output: libmdbx::Decodable<'t>
{
    fn first<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {

        let res = self.first::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			
			return mdbx_return_pair::<B, T>(pair)
		}
		Ok(None)
    }

    fn get<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.get_current::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			
			return mdbx_return_pair::<B, T>(pair)
		}
		Ok(None)
    }

    fn last<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.last::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			
			return mdbx_return_pair::<B, T>(pair)
		}
		Ok(None)
    }

    fn next<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.next::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			
			return mdbx_return_pair::<B, T>(pair)
		}
		Ok(None)
    }

    fn prev<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.prev::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			
			return mdbx_return_pair::<B, T>(pair)
		}
		Ok(None)
    }

    fn set<const B: bool>(&mut self, key: &Key<T>) -> Result<Option<Value<T>>, DBException> {

		let res: Option<<T::Value as Encode>::Output> = match key {
			Key::Type(key) => {
				let encoded_key = &key.encode()?;
				self.set::<<T::Value as Encode>::Output>(encoded_key.as_ref())?
			},
			Key::Raw(blob) => {
				self.set::<<T::Value as Encode>::Output>(blob.as_ref())?
			},
		};

		if let Some(blob) = res {
			
			return mdbx_return_value::<B, T>(blob)
		}
		Ok(None)
    }
}


/// Implementation of the DupCursor trait for mdbx's Cursors
impl<'t, T, R> crate::database::transaction::DupCursor<'t, T> for Cursor<'t, R>
where
    R: TransactionKind,
    T: DupTable,
	<T::Key as Encode>::Output: libmdbx::Decodable<'t>,
	<T::Value as Encode>::Output: libmdbx::Decodable<'t>
{
    fn first_dup<const B: bool>(&mut self) -> Result<Option<SubPair<T>>, DBException> {
        let res = self.first_dup::<<(T::SubKey,T::Value) as Encode>::Output>()?;
        if let Some(subpair) = res {
			
			return mdbx_return_subpair::<B, T>(subpair)
		}
		Ok(None)
    }

	// TODO refactor this horrible code and find a way to not have to decode then encode for blob
    fn get_dup<const B: bool>(&mut self, key: &Key<T>, subkey: &SubKey<T>) 
	-> Result<Option<Value<T>>, DBException> {
		match (key,subkey) {
			// If the supplied keys and subkeys have to be encoded
			(Key::Type(key), SubKey::Type(subkey)) => {
				let (key, subkey) = (key.encode()?, subkey.encode()?);
				let res = self.get_both_range::<<(T::SubKey,T::Value) as Encode>::Output>(key.as_ref(), subkey.as_ref())?;
				if let Some(subpair) = res {
					// If blob is needed
					if B {
						let value = <(T::SubKey,T::Value) as Decode>::decode(&subpair)?.1;
						Ok(Some(Value::Raw(value.encode()?)))
					} else {
					// If type is needed
						Ok(Some(Value::Type(<(T::SubKey,T::Value) as Decode>::decode(&subpair)?.1)))
					}
				} else {
					Ok(None)
				}
			},
			// If the supplied keys and subkeys are already encoded
			(Key::Raw(key), SubKey::Raw(subkey)) => {
				let res = self.get_both_range::<<(T::SubKey,T::Value) as Encode>::Output>(key.as_ref(), subkey.as_ref())?;
				if let Some(subpair) = res {
					// If blob is needed
					if B {
						let value = <(T::SubKey,T::Value) as Decode>::decode(&subpair)?.1;
						Ok(Some(Value::Raw(value.encode()?)))
					} else {
					// If type is needed
						Ok(Some(Value::Type(<(T::SubKey,T::Value) as Decode>::decode(&subpair)?.1)))
					}
				} else {
					Ok(None)
				}
			},
			_ => {
				unreachable!()
			}
		}
    }

    fn last_dup<const B: bool>(&mut self) -> Result<Option<SubPair<T>>, DBException> {
        let res = self.first_dup::<<(T::SubKey,T::Value) as Encode>::Output>()?;
        if let Some(subpair) = res {
			
			return mdbx_return_subpair::<B, T>(subpair)
		}
		Ok(None)
    }

    fn next_dup<const B: bool>(&mut self) -> Result<Option<FullPair<T>>, DBException> {
        let res = self.next_dup::<<T::Key as Encode>::Output, <(T::SubKey,T::Value) as Encode>::Output>()?;
		if let Some(fullpair) = res {
			
			return mdbx_return_fullpair::<B, T>(fullpair)
		}
		Ok(None)
    }

    fn prev_dup<const B: bool>(&mut self) -> Result<Option<FullPair<T>>, DBException> {
        let res = self.prev_dup::<<T::Key as Encode>::Output, <(T::SubKey,T::Value) as Encode>::Output>()?;
		if let Some(fullpair) = res {
			
			return mdbx_return_fullpair::<B, T>(fullpair)
		}
		Ok(None)
    }
}