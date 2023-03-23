use libmdbx::{RO, RW, DatabaseKind, TransactionKind, WriteFlags, Cursor};
use monero::consensus::Encodable;

use crate::{
	database::Database,
	error::{DB_FAILURES, DB_FULL, DB_SERIAL},
	table::{Table, DupTable},
	transaction::{Transaction, WriteTransaction},
};

impl From<libmdbx::Error> for DB_FAILURES {
	fn from(err: libmdbx::Error) -> Self {
		use libmdbx::Error;
		match err {
			Error::PageFull => DB_FAILURES::Full(DB_FULL::Page),
			Error::CursorFull => DB_FAILURES::Full(DB_FULL::Cursor),
			Error::ReadersFull => DB_FAILURES::Full(DB_FULL::ReadTx),
			Error::TxnFull => DB_FAILURES::Full(DB_FULL::WriteTx),
			Error::PageNotFound => DB_FAILURES::PageNotFound,
			Error::Corrupted => DB_FAILURES::PageCorrupted,
			Error::Panic => DB_FAILURES::Panic,
			Error::KeyExist => DB_FAILURES::KeyAlreadyExist,
			Error::NotFound => DB_FAILURES::KeyNotFound,
			Error::NoData => DB_FAILURES::DataNotFound,
			Error::TooLarge => DB_FAILURES::DataSizeLimit,
			Error::Other(errno) => DB_FAILURES::Undefined(errno),
			_ => DB_FAILURES::Undefined(0),
		}
	}
}

#[doc="`mdbx_encode_consensus` is a macro allocating a vector (with the given identifier) in which its supplied object is serialized using monero-rs' `consensus_encode(&mut $y)` function. </br>Return `Err(DB_FAILURES::SerializeIssue(DB_SERIAL::ConsensusEncode))` in the caller block if the function failed to encode the specified types"]
macro_rules! mdbx_encode_consensus {
    	( $x:ident, $y:ident ) => {
		let mut $y: Vec<u8> = Vec::new();
		if let Err(_) = $x.consensus_encode(&mut $y) {
			return Err(DB_FAILURES::SerializeIssue(DB_SERIAL::ConsensusEncode));
		}
    	};
}

#[doc="`mdbx_decode_consensus` is a macro defining a new variable (with its specified identifier and type) in which the supplied bytes will be deserialized using `monero::consensus::deserialize<$g>(&$x)` function. </br>Return `Err(DB_FAILURES::SerializeIssue(DB_SERIAL::ConsensusDecode))` in the caller block if the function failed to decode the supplied bytes"]
macro_rules! mdbx_decode_consensus {
    	( $x:expr, $y:ident as $g:ty ) => {
		let $y : $g;
		if let Ok(d) = monero::consensus::deserialize::<$g>(&$x) {
			$y = d;
		} else {
			return Err(DB_FAILURES::SerializeIssue(DB_SERIAL::ConsensusDecode($x)));
		}
    	};
}

#[doc="`mdbx_open_table` is a simple macro used for clarity. It try to open the table, and return a `DB_FAILURES` if it failed."]
macro_rules! mdbx_open_table {
    ($s:ident, $t:ident) => {
	let $t = $s.open_table(Some(T::TABLE_NAME))
				.map_err(std::convert::Into::<DB_FAILURES>::into)?;
    };
}

#[doc="`cursor_pair_decode` is a macro defining a conditional return used in (almost) every cursor functions. </br>If a pair of key/value effectively exist from the cursor, the two values are decoded using `mdbx_decode_consensus` function (this can cause a `DB_FAILURES::SerializeIssue`) and then returned. If no such pair exist, it simply return `Ok(None)`"]
macro_rules! cursor_pair_decode {
    	( $x:ident ) => {
		if let Some($x) = $x {
			mdbx_decode_consensus!($x.0, decoded_key as T::Key);
			mdbx_decode_consensus!($x.1, decoded_value as T::Value);
			return Ok(Some((decoded_key,decoded_value)))
		} else {
			Ok(None)
		}
    	};
}

impl<'a, E> Database<'a> for libmdbx::Database<E>
where
	E: DatabaseKind,
{
	type TX = libmdbx::Transaction<'a, RO, E>;
	type TXMut = libmdbx::Transaction<'a, RW, E>;
	type Error = libmdbx::Error;

	fn tx(&'a self) -> Result<Self::TX, Self::Error> {
		self.begin_ro_txn()
	}

	fn tx_mut(&'a self) -> Result<Self::TXMut, Self::Error> {
		self.begin_rw_txn()
	}
}

impl<'a,T,R> crate::transaction::Cursor<'a, T> for Cursor<'a, R> 
where 
	T: Table,
	R: TransactionKind
{
    	fn first(&mut self) -> Result<Option<(T::Key, T::Value)>,DB_FAILURES> {
        	let pair = self.first::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;

		cursor_pair_decode!(pair)
    	}

    	fn get_cursor(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		let pair = self.get_current::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;

		cursor_pair_decode!(pair)
		
    	}

    	fn last(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		let pair = self.last::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
				
		cursor_pair_decode!(pair)
    	}

    	fn  next(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		let pair = self.next::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
			
		cursor_pair_decode!(pair)
    	}

    	fn prev(&mut self) -> Result<Option<(<T as Table>::Key,<T as Table>::Value)>,DB_FAILURES> {
        	let pair = self.prev::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
			
		cursor_pair_decode!(pair)
    	}

    	fn set(&mut self, key: T::Key) -> Result<Option<<T as Table>::Value>,DB_FAILURES> {
		mdbx_encode_consensus!(key, encoded_key);

		let value = self.set::<Vec<u8>>(&encoded_key)
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			mdbx_decode_consensus!(value, decoded_value as T::Value);
			return Ok(Some(decoded_value))
		}
		Ok(None)
	}
}

impl<'t,T,R> crate::transaction::DupCursor<'t,T> for Cursor<'t,R> 
where 
	R: TransactionKind,
	T: DupTable,
{
    	fn first_dup(&mut self) -> Result<Option<T::Value>,DB_FAILURES> {
        	let value = self.first_dup::<Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			mdbx_decode_consensus!(value, decoded_value as T::Value);
			return Ok(Some(decoded_value))
		}
		Ok(None)
    	}

	fn get_dup(&mut self, key: T::Key, value: T::Value) -> Result<Option<<T>::Value>,DB_FAILURES> {
		mdbx_encode_consensus!(key, encoded_key);
		mdbx_encode_consensus!(value, encoded_value);

		let value = self.get_both::<Vec<u8>>(&encoded_key, &encoded_value)
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			mdbx_decode_consensus!(value, decoded_value as T::Value);
			return Ok(Some(decoded_value))
		}
		Ok(None)
	}

	fn last_dup(&mut self) -> Result<Option<<T>::Value>, DB_FAILURES> {
		let value = self.last_dup::<Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(value) = value {
			mdbx_decode_consensus!(value, decoded_value as T::Value);
			return Ok(Some(decoded_value))
		}
		Ok(None)
	}

	fn next_dup(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES> {
        	let pair = self.next_dup::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(pair) = pair {
			mdbx_decode_consensus!(pair.0, decoded_key as T::Key);
			mdbx_decode_consensus!(pair.1, decoded_value as T::Value);
			return Ok(Some((decoded_key, decoded_value)))
		}
		Ok(None)
    	}

	fn prev_dup(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES> {
        	let pair = self.prev_dup::<Vec<u8>,Vec<u8>>()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if let Some(pair) = pair {
			mdbx_decode_consensus!(pair.0, decoded_key as T::Key);
			mdbx_decode_consensus!(pair.1, decoded_value as T::Value);
			return Ok(Some((decoded_key, decoded_value)))
		}
		Ok(None)
    	}
}

impl<'a,T> crate::transaction::WriteCursor<'a, T> for Cursor<'a, RW> 
where
	T: Table,
{
	fn put_cursor(&mut self, key: <T as Table>::Key, value: <T as Table>::Value) -> Result<(),DB_FAILURES> {
        	mdbx_encode_consensus!(key, encoded_key);
		mdbx_encode_consensus!(value, encoded_value);

		self.put(&encoded_key, &encoded_value, WriteFlags::empty())
			.map_err(Into::into)
    	}

    	fn del(&mut self) -> Result<(),DB_FAILURES> {
        	
		self.del(WriteFlags::empty()).map_err(Into::into)
    	}
}

impl<'a,T> crate::transaction::DupWriteCursor<'a ,T> for Cursor<'a ,RW>
where
	T: DupTable,
{
    	fn del_nodup(&mut self) -> Result<(),DB_FAILURES> {
        	
		self.del(WriteFlags::NO_DUP_DATA).map_err(Into::into)
    	}
}

// Yes it doesn't work
impl<'a, E, R: TransactionKind> Transaction<'a> for libmdbx::Transaction<'_, R, E>
where
	E: DatabaseKind,
{
	type Cursor<T: Table> = Cursor<'a, R>;
	type DupCursor<T: DupTable> = Cursor<'a, R>;

	fn get<T: Table>(&self, key: T::Key) -> Result<Option<T::Value>, DB_FAILURES> {

		mdbx_open_table!(self, table);

		mdbx_encode_consensus!(key, encoded_key);
		let value = self.get::<Vec<u8>>(&table, &encoded_key).map_err(std::convert::Into::<DB_FAILURES>::into)?;
		if let Some(value) = value {
			mdbx_decode_consensus!(value, decoded_value as T::Value);
			return Ok(Some(decoded_value))
		}
		Ok(None)
	}

	fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DB_FAILURES> {
		mdbx_open_table!(self, table);

		self.cursor(&table).map_err(Into::into)
	}

	fn commit(self) -> Result<(), DB_FAILURES> {
		let b = self.commit()
			.map_err(std::convert::Into::<DB_FAILURES>::into)?;
		
		if b { Ok(()) } 
		else { Err(DB_FAILURES::FailedToCommit) }
	}

	fn cursor_dup<T: DupTable>(&self) -> Result<Self::DupCursor<T>,DB_FAILURES> {
        	mdbx_open_table!(self, table);

		self.cursor(&table).map_err(Into::into)
    	}
	
}

impl<'a, E> WriteTransaction<'a> for libmdbx::Transaction<'a, RW, E>
where
	E: DatabaseKind,
{
	type WriteCursor<T: Table> = Cursor<'a, RW>;
	type DupWriteCursor<T: DupTable> = Cursor<'a, RW>;

	fn put<T: Table>(&self, key: &T::Key, value: &T::Value) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self, table);

		mdbx_encode_consensus!(key, encoded_key);
		mdbx_encode_consensus!(value, encoded_value);

		self.put(&table, encoded_key, encoded_value, WriteFlags::empty()).map_err(Into::into)
	}

	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self, table);

		mdbx_encode_consensus!(key, encoded_key);
		if let Some(value) = value {
			mdbx_encode_consensus!(value, encoded_value);
			
			return self.del(&table, encoded_key, Some(encoded_value.as_slice()))
				.map(|_| ()).map_err(Into::into);
		}
		self.del(&table, encoded_key, None).map(|_| ()).map_err(Into::into)		
	}

	fn clear<T: Table>(&self) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self, table);
			
		self.clear_table(&table).map_err(Into::into)
	}

	fn write_cursor<T: Table>(&self) -> Result<Self::WriteCursor<T>, DB_FAILURES> {
		mdbx_open_table!(self, table);

		self.cursor(&table).map_err(Into::into)
	}
}
