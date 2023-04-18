//! ### MDBX implementation
//! This module contains the implementation of all the database traits for the MDBX storage engine.
//! This include basic transactions methods, cursors and errors conversion.

use std::ops::Range;
use libmdbx::{RO, RW, DatabaseKind, TransactionKind, WriteFlags, Cursor, Geometry, PageSize, Mode, SyncMode, DatabaseFlags, TableFlags};
use crate::{
	database::Database,
	error::{DB_FAILURES, DB_SERIAL},
	table::{Table, DupTable, self},
	transaction::{Transaction, WriteTransaction}, BINCODE_CONFIG,
};

// Constant used in mdbx implementation
const MDBX_DEFAULT_SYNC_MODE   : SyncMode = SyncMode::Durable;
const MDBX_MAX_MAP_SIZE: usize = 4*1024usize.pow(3); // 4TB
const MDBX_GROWTH_STEP : isize = 100*1024isize.pow(2); // 100MB
const MDBX_PAGE_SIZE   : Option<PageSize> = None;
const MDBX_GEOMETRY: Geometry<Range<usize>> = Geometry { size: Some(0..MDBX_MAX_MAP_SIZE), growth_step: Some(MDBX_GROWTH_STEP), shrink_threshold: None, page_size: MDBX_PAGE_SIZE };

/// [`mdbx_decode`] is a function which the supplied bytes will be deserialized using `bincode::decode_from_slice(src, BINCODE_CONFIG)` 
/// function. Return `Err(DB_FAILURES::SerializeIssue(DB_SERIAL::BincodeDecode(err)))` if it failed to decode the value. It is used for clarity purpose.
fn mdbx_decode<T: bincode::Decode>(src: &[u8]) -> Result<(T, usize), DB_FAILURES> {
	bincode::decode_from_slice(src, BINCODE_CONFIG)
	    	.map_err(|e| DB_FAILURES::SerializeIssue(DB_SERIAL::BincodeDecode(e)))
}

/// [`mdbx_encode`] is a function that serialize a given value into a vector using `bincode::encode_to_vec(src, BINCODE_CONFIG)`
/// function. Return `Err(DB_FAILURES::SerializeIssue(DB_SERIAL::BincodeEncode(err)))` if it failed to encode the value. It is used for clarity purpose.
fn mdbx_encode<T: bincode::Encode>(src: &T) -> Result<Vec<u8>, DB_FAILURES> {
	bincode::encode_to_vec(src, BINCODE_CONFIG)
		.map_err(|e| DB_FAILURES::SerializeIssue(DB_SERIAL::BincodeEncode(e)))
}

/// [`mdbx_open_table`] is a simple function used for syntax clarity. It try to open the table, and return a `DB_FAILURES` if it failed.
fn mdbx_open_table<'db, K: TransactionKind, E: DatabaseKind, T: Table>(tx: &'db libmdbx::Transaction<'db,K,E>) -> Result<libmdbx::Table,DB_FAILURES> {
	tx.open_table(Some(T::TABLE_NAME))
		.map_err(std::convert::Into::<DB_FAILURES>::into)
}

/// [`cursor_pair_decode`] is a function defining a conditional return used in (almost) every cursor functions. If a pair of key/value effectively exist from the cursor, 
/// the two values are decoded using `mdbx_decode` function. Return `Err(DB_FAILURES::SerializeIssue(DB_SERIAL::BincodeEncode(err)))` if it failed to encode the value. 
/// It is used for clarity purpose.
fn cursor_pair_decode<L: bincode::Decode, R: bincode::Decode>(pair: Option<(Vec<u8>,Vec<u8>)>) -> Result<Option<(L,R)>,DB_FAILURES> {
	if let Some(pair) = pair {
		let decoded_key = mdbx_decode(pair.0.as_slice())?;
		let decoded_value = mdbx_decode(pair.1.as_slice())?;
		Ok(Some((decoded_key.0,decoded_value.0)))
	} else {
		Ok(None)
	}
}

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
			.set_flags(DatabaseFlags::from(Mode::ReadWrite{sync_mode:MDBX_DEFAULT_SYNC_MODE}))
			.set_geometry(MDBX_GEOMETRY)
			.set_max_readers(32)
			.set_max_tables(15)
			.open(path.as_path())?;

		Ok(db)
	}

	// Open each tables to verify if the database is complete.
	fn check(&'a self) -> Result<(), Self::Error> {
		let ro_tx = self.begin_ro_txn()?;
		// ----- BLOCKS -----
		ro_tx.open_table(Some(table::blockhash::TABLE_NAME))?;
		ro_tx.open_table(Some(table::blockmetadata::TABLE_NAME))?;
		ro_tx.open_table(Some(table::blocks::TABLE_NAME))?;
		ro_tx.open_table(Some(table::blockhfversion::TABLE_NAME))?;
		ro_tx.open_table(Some(table::altblock::TABLE_NAME))?;
		// ------ TXNs ------
		ro_tx.open_table(Some(table::txsprefix::TABLE_NAME))?;
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
		rw_tx.create_table(Some(table::blockhash::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::blockmetadata::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::blocks::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::blockhfversion::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::altblock::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		// ------ TXNs ------
		rw_tx.create_table(Some(table::txsprefix::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::txsprunable::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::txsprunablehash::TABLE_NAME), TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT)?;
		rw_tx.create_table(Some(table::txsprunabletip::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		rw_tx.create_table(Some(table::txsoutputs::TABLE_NAME), TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT)?;
		rw_tx.create_table(Some(table::txsidentifier::TABLE_NAME), TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT)?;
		// ---- OUTPUTS -----
		rw_tx.create_table(Some(table::prerctoutputmetadata::TABLE_NAME), TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT)?;
		rw_tx.create_table(Some(table::outputmetadata::TABLE_NAME), TableFlags::INTEGER_KEY)?;
		// ---- SPT KEYS ----
		rw_tx.create_table(Some(table::spentkeys::TABLE_NAME), TableFlags::INTEGER_KEY | TableFlags::DUP_FIXED | TableFlags::DUP_SORT)?;
		// --- PROPERTIES ---
		rw_tx.create_table(Some(table::properties::TABLE_NAME), TableFlags::INTEGER_KEY)?;

		rw_tx.commit()?;
		Ok(())
	}
}

// Implementation of the Cursor trait for mdbx's Cursors
impl<'a,T,R> crate::transaction::Cursor<'a, T> for Cursor<'a, R> 
where 
	T: Table,
	R: TransactionKind
{
    fn first(&mut self) -> Result<Option<(T::Key, T::Value)>,DB_FAILURES> {
        	let pair = self.first::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;

		cursor_pair_decode(pair)
    }

    fn get_cursor(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		let pair = self.get_current::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;

		cursor_pair_decode(pair)
		
    }

    fn last(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		let pair = self.last::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
				
		cursor_pair_decode(pair)
    }

    fn  next(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		let pair = self.next::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
			
		cursor_pair_decode(pair)
    }

    fn prev(&mut self) -> Result<Option<(<T as Table>::Key,<T as Table>::Value)>,DB_FAILURES> {
        	let pair = self.prev::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
			
		cursor_pair_decode(pair)
    }

    fn set(&mut self, key: &T::Key) -> Result<Option<<T as Table>::Value>,DB_FAILURES> {
		let encoded_key = mdbx_encode(key)?;

		let value = self.set::<Vec<u8>>(&encoded_key)
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			return Ok(Some(mdbx_decode(value.as_slice())?.0))
		}
		Ok(None)
	}
}

// Implementation of the DupCursor trait for mdbx's Cursors
impl<'t,T,R> crate::transaction::DupCursor<'t,T> for Cursor<'t,R> 
where 
	R: TransactionKind,
	T: DupTable,
{
    fn first_dup(&mut self) -> Result<Option<(T::SubKey,T::Value)>,DB_FAILURES> {
        let value = self.first_dup::<Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			return Ok(Some(mdbx_decode(value.as_slice())?.0))
		}
		Ok(None)
    }

	fn get_dup(&mut self, key: &T::Key, subkey: &T::SubKey) -> Result<Option<<T>::Value>,DB_FAILURES> {
		let (encoded_key, encoded_subkey) = (mdbx_encode(key)?, mdbx_encode(subkey)?);

		let value = self.get_both::<Vec<u8>>(&encoded_key, &encoded_subkey)
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			return Ok(Some(mdbx_decode(value.as_slice())?.0))
		}
		Ok(None)
	}

	fn last_dup(&mut self) -> Result<Option<(T::SubKey,T::Value)>, DB_FAILURES> {
		let value = self.last_dup::<Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			return Ok(Some(mdbx_decode(value.as_slice())?.0))
		}
		Ok(None)
	}

	fn next_dup(&mut self) -> Result<Option<(T::Key, (T::SubKey, T::Value))>, DB_FAILURES> {
        let pair = self.next_dup::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(pair) = pair {
			let (decoded_key, decoded_value) = (mdbx_decode(pair.0.as_slice())?, mdbx_decode(pair.1.as_slice())?);			
			return Ok(Some((decoded_key.0, decoded_value.0)))
		}
		Ok(None)
    }

	fn prev_dup(&mut self) -> Result<Option<(T::Key, (T::SubKey, T::Value))>, DB_FAILURES> {
        let pair = self.prev_dup::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(pair) = pair {
			let (decoded_key, decoded_value) = (mdbx_decode(pair.0.as_slice())?, mdbx_decode(pair.1.as_slice())?);			
			return Ok(Some((decoded_key.0, decoded_value.0)))
		}
		Ok(None)
    }
}

// Implementation of the WriteCursor trait for mdbx's Cursors in RW permission
impl<'a,T> crate::transaction::WriteCursor<'a, T> for Cursor<'a, RW> 
where
	T: Table,
{
	fn put_cursor(&mut self, key: &T::Key, value: &T::Value) -> Result<(),DB_FAILURES> {
        let (encoded_key, encoded_value) = (mdbx_encode(key)?, mdbx_encode(value)?);

		self.put(&encoded_key, &encoded_value, WriteFlags::empty())
			.map_err(Into::into)
    }
	
    fn del(&mut self) -> Result<(),DB_FAILURES> {
    	
		self.del(WriteFlags::empty()).map_err(Into::into)
    }
}

// Implementation of the DupWriteCursor trait for mdbx's Cursors in RW permission
impl<'a,T> crate::transaction::DupWriteCursor<'a ,T> for Cursor<'a ,RW>
where
	T: DupTable,
{
    fn del_nodup(&mut self) -> Result<(),DB_FAILURES> {
        	
		self.del(WriteFlags::NO_DUP_DATA).map_err(Into::into)
    }

    fn put_cursor_dup(&mut self, key: &<T>::Key, subkey: &<T as DupTable>::SubKey, value: &<T>::Value) -> Result<(),DB_FAILURES> {
		let (encoded_key, mut encoded_subkey, mut encoded_value) = (mdbx_encode(key)?, mdbx_encode(subkey)?, mdbx_encode(value)?);
		encoded_subkey.append(&mut encoded_value);

		self.put(encoded_key.as_slice(), encoded_subkey.as_slice(), WriteFlags::empty())
			.map_err(Into::into)
    }
}

// Implementation of the Transaction trait for mdbx's Transactions
impl<'a, E, R: TransactionKind> Transaction<'a> for libmdbx::Transaction<'_, R, E>
where
	E: DatabaseKind,
{
	type Cursor<T: Table> = Cursor<'a, R>;
	type DupCursor<T: DupTable> = Cursor<'a, R>;

	fn get<T: Table>(&self, key: &T::Key) -> Result<Option<T::Value>, DB_FAILURES> {

		let table = mdbx_open_table::<_,_,T>(self)?;

		let encoded_key = mdbx_encode(key)?;

		let value = self.get::<Vec<u8>>(&table, &encoded_key).map_err(std::convert::Into::<DB_FAILURES>::into)?;
		if let Some(value) = value {
			return Ok(Some(mdbx_decode(value.as_slice())?.0))
		}
		Ok(None)
	}

	fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DB_FAILURES> {
		let table = mdbx_open_table::<_,_,T>(self)?;

		self.cursor(&table).map_err(Into::into)
	}

	fn commit(self) -> Result<(), DB_FAILURES> {
		let b = self.commit()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if b { Ok(()) } 
		else { Err(DB_FAILURES::FailedToCommit) }
	}

	fn cursor_dup<T: DupTable>(&self) -> Result<Self::DupCursor<T>,DB_FAILURES> {
    	let table = mdbx_open_table::<_,_,T>(self)?;

		self.cursor(&table).map_err(Into::into)
    }

	fn num_entries<T: Table>(&self) -> Result<usize, DB_FAILURES> {
		let table = mdbx_open_table::<_,_,T>(self)?;
        let stat = self.table_stat(&table)?;

		Ok(stat.entries())
    }
	
}

// Implementation of the Transaction trait for mdbx's Transactions with RW permissions
impl<'a, E> WriteTransaction<'a> for libmdbx::Transaction<'a, RW, E>
where
	E: DatabaseKind,
{
	type WriteCursor<T: Table> = Cursor<'a, RW>;
	type DupWriteCursor<T: DupTable> = Cursor<'a, RW>;

	fn put<T: Table>(&self, key: &T::Key, value: &T::Value) -> Result<(), DB_FAILURES> {
		let table = mdbx_open_table::<_,_,T>(self)?;

		let (encoded_key, encoded_value) = (mdbx_encode(key)?, mdbx_encode(value)?);

		self.put(&table, encoded_key, encoded_value, WriteFlags::empty()).map_err(Into::into)
	}

	fn delete<T: Table>(&self, key: &T::Key, value: &Option<T::Value>) -> Result<(), DB_FAILURES> {
		let table = mdbx_open_table::<_,_,T>(self)?;

		let encoded_key = mdbx_encode(key)?;
		if let Some(value) = value {
			let encoded_value = mdbx_encode(value)?;
			
			return self.del(&table, encoded_key, Some(encoded_value.as_slice()))
				.map(|_| ()).map_err(Into::into);
		}
		self.del(&table, encoded_key, None).map(|_| ()).map_err(Into::into)		
	}

	fn clear<T: Table>(&self) -> Result<(), DB_FAILURES> {
		let table = mdbx_open_table::<_,_,T>(self)?;
			
		self.clear_table(&table).map_err(Into::into)
	}

	fn write_cursor<T: Table>(&self) -> Result<Self::WriteCursor<T>, DB_FAILURES> {
		let table = mdbx_open_table::<_,_,T>(self)?;

		self.cursor(&table).map_err(Into::into)
	}

	fn write_cursor_dup<T: DupTable>(&self) -> Result<Self::DupWriteCursor<T>, DB_FAILURES> {
        let table = mdbx_open_table::<_,_,T>(self)?;

		self.cursor(&table).map_err(Into::into)
    }
}
