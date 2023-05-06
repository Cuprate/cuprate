//! ### MDBX implementation
//! This module contains the implementation of all the database traits and methods for the MDBX storage engine.
//! This include basic transactions methods, cursors and errors conversion.

use crate::{
    database::{Database, transaction::{Transaction, WriteTransaction, Pair, SubPair, FullPair}},
	encoding::{Value, Encode, Decode, Buffer, Error},
    table::{self, DupTable, Table},
    error::DBException,
};
use libmdbx::{
    Cursor, DatabaseFlags, DatabaseKind, Geometry, Mode, PageSize, SyncMode, TableFlags,
    TransactionKind, WriteFlags, RO, RW,
};
use std::{ops::Range, io::Write};

// ------------| Constant used in mdbx implementation |-------------------

const MDBX_MAX_MAP_SIZE: usize = 4 * 1024usize.pow(3); // 4TB
const MDBX_GROWTH_STEP: isize = 100 * 1024isize.pow(2); // 100MB
const MDBX_PAGE_SIZE: Option<PageSize> = None; // Subject to change
const MDBX_GEOMETRY: Geometry<Range<usize>> = Geometry {
    size: Some(0..MDBX_MAX_MAP_SIZE),
    growth_step: Some(MDBX_GROWTH_STEP),
    shrink_threshold: None,
    page_size: MDBX_PAGE_SIZE,
};

// ------------| Convenient functions |-------------------

/// [`mdbx_return_pair`] is a convenient implementation used at the end of cursors methods to either return the raw pair or decoded pair from database
fn mdbx_return_pair<const B: bool, T: Table>(pair: (<T::Key as Encode>::Output, <T::Value as Encode>::Output)) -> Result<Option<Pair<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		Ok(Some((<T::Key as Decode>::decode(&pair.0)?,Value::Raw(pair.1))))
	} else {
		// Extracting then decoding the bytes form the database
		Ok(Some((
			<T::Key as Decode>::decode(&pair.0)?,
			Value::Type(<T::Value as Decode>::decode(&pair.1)?)
		)))
	}
}

#[allow(clippy::type_complexity)]
/// [`extracting_subpair_blob`] is a convenient function to extract the (SubKey,Value) blob from the database into (Subkey's blob,Value's blob)
fn extracting_subpair_blob<T: DupTable>(subpair: <(T::SubKey, T::Value) as Encode>::Output) -> Result<(<T::SubKey as Encode>::Output,<T::Value as Encode>::Output), DBException> {
	// Generating buffers
	let mut subkey = <<T::SubKey as Encode>::Output as Buffer>::new();
	let mut value = <<T::Value as Encode>::Output as Buffer>::new();

	// Spliting the memory into two pointers
	let split = subpair.split_at(std::mem::size_of::<T::SubKey>());

	// Writing into buffers
	subkey.as_mut().write_all(split.0).map_err(Error::IoError)?;
	value.as_mut().write_all(split.1).map_err(Error::IoError)?;
	Ok((subkey,value))
}

/// [`mdbx_return_subpair`] is a convenient implementation used at the end of cursors methods to either return the raw pair or decoded pair from database
fn mdbx_return_subpair<const B: bool, T: DupTable>(subpair: <(T::SubKey, T::Value) as Encode>::Output) -> Result<Option<SubPair<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		let (subkey,value) = extracting_subpair_blob::<T>(subpair)?;
		Ok(Some(SubPair::Raw(subkey,value)))
	} else {
		// Extracting then decoding the bytes form the database
		let (subkey,value) = <(T::SubKey, T::Value) as Decode>::decode(&subpair)?;
		Ok(Some(
			SubPair::<T>::Type(
				subkey,
				value
			)
		))
	}
}

#[allow(clippy::type_complexity)]
/// [`mdbx_return_subpair`] is a convenient implementation used at the end of cursors methods to either return the raw pair or decoded pair from database
fn mdbx_return_fullpair<const B: bool, T: DupTable>(fullpair: (<T::Key as Encode>::Output, <(T::SubKey, T::Value) as Encode>::Output)) -> Result<Option<FullPair<T>>, DBException> {
	if B {
		// Directly extracting the bytes from the database
		let (subkey,value) = extracting_subpair_blob::<T>(fullpair.1)?;
		Ok(Some((<T::Key as Decode>::decode(&fullpair.0)?, SubPair::Raw(subkey,value))))
	} else {
		// Extracting then decoding the bytes form the database
		let (subkey,value) = <(T::SubKey, T::Value) as Decode>::decode(&fullpair.1)?;
		let key = <T::Key as Decode>::decode(&fullpair.0)?;
		Ok(Some(
			(
				key,
				SubPair::<T>::Type(
					subkey,
					value
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

// ------------| Database Trait implementation |-----------------------------

// Implementation of the database trait with mdbx types
impl<'a, E> Database<'a> for libmdbx::Database<E>
where
    E: DatabaseKind,
{
    type TX = libmdbx::Transaction<'a, RO, E>;
    type TXMut = libmdbx::Transaction<'a, RW, E>;
    type Error = libmdbx::Error;

    // Open a Read-Only transaction
    fn tx(&'a self) -> Result<Self::TX, Self::Error> {
        self.begin_ro_txn()
    }

    // Open a Read-Write transaction
    fn tx_mut(&'a self) -> Result<Self::TXMut, Self::Error> {
        self.begin_rw_txn()
    }

    // Open the database with the given path
    fn open(path: std::path::PathBuf) -> Result<Self, Self::Error> {
        let db: libmdbx::Database<E> = libmdbx::Database::new()
            .set_flags(DatabaseFlags::from(Mode::ReadWrite {
                sync_mode: SyncMode::default(),
            }))
            .set_geometry(MDBX_GEOMETRY)
            .set_max_readers(32)
            .set_max_tables(15)
            .open(path.as_path())?;

        Ok(db)
    }

    // Open each tables to verify if the database is complete.
    fn check_is_correctly_built(&'a self) -> Result<(), Self::Error> {
        let ro_tx = self.begin_ro_txn()?;
        // ----- BLOCKS -----
        ro_tx.open_table(Some(table::blockhash::TABLE_NAME))?;
        ro_tx.open_table(Some(table::blockmetadata::TABLE_NAME))?;
        ro_tx.open_table(Some(table::blocks::TABLE_NAME))?;
        ro_tx.open_table(Some(table::altblock::TABLE_NAME))?;
        // ------ TXNs ------
        ro_tx.open_table(Some(table::txspruned::TABLE_NAME))?;
        ro_tx.open_table(Some(table::txsprunablehash::TABLE_NAME))?;
        ro_tx.open_table(Some(table::txsprunabletip::TABLE_NAME))?;
        ro_tx.open_table(Some(table::txsprunable::TABLE_NAME))?;
        ro_tx.open_table(Some(table::txsoutputs::TABLE_NAME))?;
        ro_tx.open_table(Some(table::txsidentifier::TABLE_NAME))?;
        // ---- OUTPUTS -----
        ro_tx.open_table(Some(table::prerctoutputmetadata::TABLE_NAME))?;
        ro_tx.open_table(Some(table::outputmetadata::TABLE_NAME))?;
        // ---- SPT KEYS ----
        ro_tx.open_table(Some(table::spentkeys::TABLE_NAME))?;
        // --- PROPERTIES ---
        ro_tx.open_table(Some(table::properties::TABLE_NAME))?;

        Ok(())
    }

    // Construct the table of the database
    fn build(&'a self) -> Result<(), Self::Error> {
        let rw_tx = self.begin_rw_txn()?;

        // Constructing the tables
        // ----- BLOCKS -----
        rw_tx.create_table(
            Some(table::blockhash::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        rw_tx.create_table(
            Some(table::blockmetadata::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        rw_tx.create_table(Some(table::blocks::TABLE_NAME), TableFlags::INTEGER_KEY)?;
        rw_tx.create_table(Some(table::altblock::TABLE_NAME), TableFlags::INTEGER_KEY)?;
        // ------ TXNs ------
        rw_tx.create_table(Some(table::txspruned::TABLE_NAME), TableFlags::INTEGER_KEY)?;
        rw_tx.create_table(
            Some(table::txsprunable::TABLE_NAME),
            TableFlags::INTEGER_KEY,
        )?;
        rw_tx.create_table(
            Some(table::txsprunablehash::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        rw_tx.create_table(
            Some(table::txsprunabletip::TABLE_NAME),
            TableFlags::INTEGER_KEY,
        )?;
        rw_tx.create_table(
            Some(table::txsoutputs::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        rw_tx.create_table(
            Some(table::txsidentifier::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        // ---- OUTPUTS -----
        rw_tx.create_table(
            Some(table::prerctoutputmetadata::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        rw_tx.create_table(
            Some(table::outputmetadata::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        // ---- SPT KEYS ----
        rw_tx.create_table(
            Some(table::spentkeys::TABLE_NAME),
            TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT,
        )?;
        // --- PROPERTIES ---
        rw_tx.create_table(Some(table::properties::TABLE_NAME), TableFlags::INTEGER_KEY)?;

        rw_tx.commit()?;
        Ok(())
    }
}

// ------------| Transaction/Cursor Trait Implementation |-------------------

// Implementation of the Cursor trait for mdbx's Cursors
impl<'t, T, R> crate::database::transaction::Cursor<'t, T> for Cursor<'t, R>
where
    T: Table,
    R: TransactionKind,
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

    fn set<const B: bool>(&mut self, key: &T::Key) -> Result<Option<Value<T>>, DBException> {
		let res = self.set::<<T::Value as Encode>::Output>(key.encode()?.as_ref())?;
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
{
    fn first_dup<const B: bool>(&mut self) -> Result<Option<SubPair<T>>, DBException> {
        let res = self.first_dup::<<(T::SubKey,T::Value) as Encode>::Output>()?;
        if let Some(subpair) = res {
			
			return mdbx_return_subpair::<B, T>(subpair)
		}
		Ok(None)
    }

    fn get_dup<const B: bool>(&mut self, key: &T::Key, subkey: &T::SubKey) 
	-> Result<Option<Value<T>>, DBException> {
		let (key, subkey) = (key.encode()?, subkey.encode()?);
		let res = self.get_both_range::<<(T::SubKey,T::Value) as Encode>::Output>(key.as_ref(), subkey.as_ref())?
			.filter(|data| data.starts_with(subkey.as_ref())); // Checking if this is the correct data being returned or a greater one

		if let Some(subpair) = res {
			// If blob is needed
			if B {
				let (_,value) = extracting_subpair_blob::<T>(subpair)?;
				return Ok(Some(Value::Raw(value)))
			} else {
			// If type is needed
				return Ok(Some(Value::Type(<(T::SubKey,T::Value) as Decode>::decode(&subpair)?.1)))
			}
		}
		Ok(None)
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

/// Implementation of the WriteCursor trait for mdbx's Cursors in RW permission
impl<'t, T> crate::database::transaction::WriteCursor<'t, T> for Cursor<'t, RW>
where
    T: Table,
{
    fn put_cursor(&mut self, key: &T::Key, value: &Value<T>) -> Result<(), DBException> {
		match value {
			// The value is already encoded
			Value::Raw(value) => {
				Ok(self.put(key.encode()?.as_ref(), value.as_ref(), WriteFlags::empty())?)
			},
			// The value is typed
			Value::Type(value) => {
				Ok(self.put(key.encode()?.as_ref(), value.encode()?.as_ref(), WriteFlags::empty())?)
			}
		}
    }

    fn del(&mut self) -> Result<(), DBException> {
        
		Ok(self.del(WriteFlags::empty())?)
    }
}

/// Implementation of the DupWriteCursor trait for mdbx's Cursors in RW permission
impl<'t, T> crate::database::transaction::DupWriteCursor<'t, T> for Cursor<'t, RW>
where
    T: DupTable,
{
    fn put_cursor_dup(&mut self, key: &T::Key, subkey: &T::SubKey, value: &Value<T>) 
	-> Result<(), DBException> {
		match value {
			// The value is already encoded
			Value::Raw(value) => {
				let (key, subkey) = (key.encode()?, subkey.encode()?);

				// Assembling subkey and value
				let mut data = Vec::with_capacity(1024);
				data.copy_from_slice(subkey.as_ref());
				data.extend_from_slice(value.as_ref());

				Ok(self.put(key.as_ref(), data.as_ref(), WriteFlags::empty())?)
			},
			// The value is typed
			Value::Type(value) => {
				let (key, subkey, value) = (key.encode()?, subkey.encode()?, value.encode()?);

				// Assembling subkey and value
				let mut data = Vec::with_capacity(1024);
				data.copy_from_slice(subkey.as_ref());
				data.extend_from_slice(value.as_ref());

				Ok(self.put(key.as_ref(), data.as_ref(), WriteFlags::empty())?)
			}
		}
    }

    fn del_nodup(&mut self) -> Result<(), DBException> {
        Ok(self.del(WriteFlags::NO_DUP_DATA)?)
    }
}

/// Implementation of the Transaction trait for mdbx's Transactions
impl<'m, E, R: TransactionKind> Transaction<'m> for libmdbx::Transaction<'_, R, E>
where
    E: DatabaseKind,
{
    type Cursor<T: Table> = Cursor<'m, R>; 
    type DupCursor<T: DupTable> = Cursor<'m, R>;

    fn get<const B: bool, T: Table>(&self, key: &T::Key) -> Result<Option<Value<T>>, DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		let res = self.get::<<T::Value as Encode>::Output>(&table, key.encode()?.as_ref())?;

		if let Some(value) = res {
			if B {
				Ok(Some(Value::Raw(value)))
			} else {
				Ok(Some(Value::Type(<T::Value as Decode>::decode(&value)?)))
			}
		} else {
			Ok(None)
		}
    }

    fn commit(self) -> Result<(), DBException> {
        if self.commit()? {
            Ok(())
        } else {
            Err(DBException::FailedToCommit)
        }
    }

    fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		Ok(self.cursor(&table)?)
    }

    fn cursor_dup<T: DupTable>(&self) -> Result<Self::DupCursor<T>, DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		Ok(self.cursor(&table)?)
    }

    fn num_entries<T: Table>(&self) -> Result<usize, DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

        let stat = self.table_stat(&table)?;

        Ok(stat.entries())
    }
}

/// Implementation of the Transaction trait for mdbx's Transactions with RW permissions
impl<'m, E> WriteTransaction<'m> for libmdbx::Transaction<'m, RW, E>
where
    E: DatabaseKind,
{
    type WriteCursor<T: Table> = Cursor<'m, RW>;
    type DupWriteCursor<T: DupTable> = Cursor<'m, RW>;

    fn put<T: Table>(&self, key: &T::Key, value: &Value<T>) -> Result<(), DBException> {
		let table = self.open_table(Some(T::TABLE_NAME))?;

        match value {
			Value::Type(value) => {
				Ok(self.put(&table, key.encode()?.as_ref(), value.encode()?.as_ref(), WriteFlags::empty())?)
			},
			Value::Raw(value) => {
				Ok(self.put(&table, key.encode()?.as_ref(), value.as_ref(), WriteFlags::empty())?)
			},
		}
    }

    fn delete<T: Table>(&self, key: &T::Key, value: &Option<T::Value>) 
	-> Result<(), DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		if let Some(value) = value {
			Ok(self.del(&table, key.encode()?.as_ref(), Some(value.encode()?.as_ref())).map(|_|())?)
		} else {
			Ok(self.del(&table, key.encode()?.as_ref(), None).map(|_|())?)
		}
    }

    fn clear<T: Table>(&self) -> Result<(), DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		Ok(self.clear_table(&table)?)
    }

    fn write_cursor<T: Table>(&self) -> Result<Self::WriteCursor<T>, DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		Ok(self.cursor(&table)?)
    }

    fn write_cursor_dup<T: DupTable>(&self) -> Result<Self::DupWriteCursor<T>, DBException> {
        let table = self.open_table(Some(T::TABLE_NAME))?;

		Ok(self.cursor(&table)?)
    }
}

