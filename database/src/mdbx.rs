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

// Implementation of the Cursor trait for mdbx's Cursors
impl<'a, T, R> crate::database::transaction::Cursor<'a, T> for Cursor<'a, R>
where
    T: Table,
    R: TransactionKind,
	<<T>::Key as Encode>::Output: libmdbx::Decodable<'a>,
	<<T>::Value as Encode>::Output: libmdbx::Decodable<'a>
{
    fn first<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {

        let res = self.first::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			if B {
				// If blob needed
				return Ok(Some((Key::Raw(pair.0),Value::Raw(pair.1))))
			} else {
				// If decoded type needed
				return Ok(Some((
					Key::Type(<T::Key as Decode>::decode(&pair.0)?),
					Value::Type(<T::Value as Decode>::decode(&pair.1)?)
				)))
			}
		}
		Ok(None)
    }

    fn get<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.get_current::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			if B {
				// If blob needed
				return Ok(Some((Key::Raw(pair.0),Value::Raw(pair.1))))
			} else {
				// If decoded type needed
				return Ok(Some((
					Key::Type(<T::Key as Decode>::decode(&pair.0)?),
					Value::Type(<T::Value as Decode>::decode(&pair.1)?)
				)))
			}
		}
		Ok(None)
    }

    fn last<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.last::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			if B {
				// If blob needed
				return Ok(Some((Key::Raw(pair.0),Value::Raw(pair.1))))
			} else {
				// If decoded type needed
				return Ok(Some((
					Key::Type(<T::Key as Decode>::decode(&pair.0)?),
					Value::Type(<T::Value as Decode>::decode(&pair.1)?)
				)))
			}
		}
		Ok(None)
    }

    fn next<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.next::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			if B {
				// If blob needed
				return Ok(Some((Key::Raw(pair.0),Value::Raw(pair.1))))
			} else {
				// If decoded type needed
				return Ok(Some((
					Key::Type(<T::Key as Decode>::decode(&pair.0)?),
					Value::Type(<T::Value as Decode>::decode(&pair.1)?)
				)))
			}
		}
		Ok(None)
    }

    fn prev<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException> {
        let res = self.prev::<<<T>::Key as Encode>::Output, <<T>::Value as Encode>::Output>()?;
		if let Some(pair) = res {
			if B {
				// If blob needed
				return Ok(Some((Key::Raw(pair.0),Value::Raw(pair.1))))
			} else {
				// If decoded type needed
				return Ok(Some((
					Key::Type(<T::Key as Decode>::decode(&pair.0)?),
					Value::Type(<T::Value as Decode>::decode(&pair.1)?)
				)))
			}
		}
		Ok(None)
    }

    fn set<const B: bool>(&mut self, key: &Key<T>) -> Result<Option<Value<T>>, DBException> {

		let res: Option<<T::Value as Encode>::Output> = match key {
			Key::Type(key) => {
				let encoded_key = &key.encode()?;
				self.set::<<T::Value as Encode>::Output>(encoded_key.as_ref())
					.map_err(DBException::MDBX_Error)?
			},
			Key::Raw(blob) => {
				self.set::<<T::Value as Encode>::Output>(blob.as_ref())
					.map_err(DBException::MDBX_Error)?
			},
		};

		if let Some(blob) = res {
			if B {
				return Ok(Some(Value::Raw(blob)));
			} else {
				return Ok(Some(Value::Type(<T::Value as Decode>::decode(&blob)?)))
			}
		}
		Ok(None)
    }
}