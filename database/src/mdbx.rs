use libmdbx::{RO, RW, DatabaseKind, TransactionKind, WriteFlags, Cursor};
use monero::consensus::Encodable;

use crate::{
	database::Database,
	error::{DB_FAILURES, DB_FULL, DB_SERIAL},
	table::Table,
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

macro_rules! mdbx_encode_consensus {
    	( $x:ident, $y:ident ) => {
		let mut $y: Vec<u8> = Vec::new();
		if let Err(_) = $x.consensus_encode(&mut $y) {
			return Err(DB_FAILURES::SerializeIssue(DB_SERIAL::ConsensusEncode));
		}
    	};
}

macro_rules! mdbx_decode_consensus {
    	( $x:expr, $y:ident, $g:ty ) => {
		let $y : $g;
		if let Ok(d) = monero::consensus::deserialize::<$g>(&$x) {
			$y = d;
		} else {
			return Err(DB_FAILURES::SerializeIssue(DB_SERIAL::ConsensusDecode($x)));
		}
    	};
}

macro_rules! mdbx_open_table {
	( $s:ident | $t:ident | $e:ident, $x:block ) => {
		match $s.open_table(Some(T::TABLE_NAME)) {
			Ok($t) => $x,
			Err($e) => Err($e.into()),
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
        	match self.first::<Vec<u8>,Vec<u8>>().map_err(|e| e.into()) {
			Ok(pair) => {
				if let Some(pair) = pair {
					mdbx_decode_consensus!(pair.0, decoded_key, T::Key);
					mdbx_decode_consensus!(pair.1, decoded_value, T::Value);
					return Ok(Some((decoded_key,decoded_value)))
				}
				Ok(None)
			}
			Err(e) => Err(e),
		}
    	}

    	fn get(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		match self.get_current::<Vec<u8>,Vec<u8>>().map_err(|e| e.into()) {
			Ok(pair) => {
				if let Some(pair) = pair {
					mdbx_decode_consensus!(pair.0, decoded_key, T::Key);
					mdbx_decode_consensus!(pair.1, decoded_value, T::Value);
					return Ok(Some((decoded_key,decoded_value)))
				}
				Ok(None)
			}
			Err(e) => Err(e),
		}
    	}

    	fn last(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		match self.last::<Vec<u8>,Vec<u8>>().map_err(|e| e.into()) {
			Ok(pair) => {
				if let Some(pair) = pair {
					mdbx_decode_consensus!(pair.0, decoded_key, T::Key);
					mdbx_decode_consensus!(pair.1, decoded_value, T::Value);
					return Ok(Some((decoded_key,decoded_value)))
				}
				Ok(None)
			}
			Err(e) => Err(e),
		}
    	}

    fn  next(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
        match self.next::<Vec<u8>,Vec<u8>>().map_err(|e| e.into()) {
		Ok(pair) => {
			if let Some(pair) = pair {
				mdbx_decode_consensus!(pair.0, decoded_key, T::Key);
				mdbx_decode_consensus!(pair.1, decoded_value, T::Value);
				return Ok(Some((decoded_key,decoded_value)))
			}
			Ok(None)
		}
		Err(e) => Err(e),
	}
    }

    fn prev(&mut self) -> Result<Option<(<T as Table>::Key,<T as Table>::Value)>,DB_FAILURES> {
        match self.prev::<Vec<u8>,Vec<u8>>().map_err(|e| e.into()) {
		Ok(pair) => {
			if let Some(pair) = pair {
				mdbx_decode_consensus!(pair.0, decoded_key, T::Key);
				mdbx_decode_consensus!(pair.1, decoded_value, T::Value);
				return Ok(Some((decoded_key,decoded_value)))
			}
			Ok(None)
		}
		Err(e) => Err(e),
	}
    }

    fn set(&mut self, key: T::Key) -> Result<Option<<T as Table>::Value>,DB_FAILURES> {
	mdbx_encode_consensus!(key, encoded_key);
	match self.set::<Vec<u8>>(&encoded_key).map_err(|e| e.into()) {
   		Ok(value) => {
			if let Some(value) = value {
				mdbx_decode_consensus!(value, decoded_value, T::Value);
				return Ok(Some(decoded_value))
			}
			Ok(None)
		},
    		Err(e) => Err(e),
	}
    }
}

impl<'a,T> crate::transaction::WriteCursor<'a, T> for Cursor<'a, RW> 
where
	T: Table,
{
	fn put(&mut self, key: <T as Table>::Key, value: <T as Table>::Value) -> Result<(),DB_FAILURES> {
        	mdbx_encode_consensus!(key, encoded_key);
		mdbx_encode_consensus!(value, encoded_value);

		self.put(&encoded_key, &encoded_value, WriteFlags::empty())
			.map_err(|err| err.into())
    	}

    	fn del(&mut self) -> Result<(),DB_FAILURES> {
        	
		self.del(WriteFlags::empty()).map_err(|err| err.into())
    	}
}

// Yes it doesn't work
impl<'a, E, R: TransactionKind> Transaction<'a> for libmdbx::Transaction<'_, R, E>
where
	E: DatabaseKind,
{
	type Cursor<T: Table> = Cursor<'a, R>;

	fn get<T: Table>(&self, key: T::Key) -> Result<Option<T::Value>, DB_FAILURES> {
		mdbx_open_table!(self | table | err, {

			mdbx_encode_consensus!(key, encoded_key);
			match self.get::<Vec<u8>>(&table, &encoded_key).map_err(|err| err.into()) {
				Ok(Some(value)) => {
					mdbx_decode_consensus!(value, decoded_value, T::Value);
					Ok(Some(decoded_value))
				},
				Ok(None) => Ok(None),
				Err(err) => Err(err),
			}
		})
	}

	fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DB_FAILURES> {
		mdbx_open_table!(self | table | err, {

			self.cursor(&table).map_err(|err| err.into())
		})
	}

	fn commit(self) -> Result<(), DB_FAILURES> {
		match self.commit().map_err(|err| err.into()) {
			Ok(b) => {
				if b { Ok(()) } 
				else { Err(DB_FAILURES::FailedToCommit) }
			},
			Err(err) => Err(err),
		}
	}
	
}

impl<'a, E> WriteTransaction<'a> for libmdbx::Transaction<'a, RW, E>
where
	E: DatabaseKind,
{
	type WriteCursor<T: Table> = Cursor<'a, RW>;

	fn put<T: Table>(&self, key: &T::Key, value: &T::Value) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self | table | err, {
			mdbx_encode_consensus!(key, encoded_key);
			mdbx_encode_consensus!(value, encoded_value);

			self.put(&table, encoded_key, encoded_value, WriteFlags::empty()).map_err(|err| err.into())
		})
	}

	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self | table | err, {
			mdbx_encode_consensus!(key, encoded_key);
			if let Some(value) = value {
				mdbx_encode_consensus!(value, encoded_value);
				
				return self.del(&table, encoded_key, Some(encoded_value.as_slice()))
					.map(|_| ()).map_err(|err| err.into());
			}
			self.del(&table, encoded_key, None).map(|_| ()).map_err(|err| err.into())
		})
		
	}

	fn clear<T: Table>(&self) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self | table | err, {
			
			self.clear_table(&table).map_err(|err| err.into())
		})
	}

	fn write_cursor<T: Table>(&self) -> Result<Self::WriteCursor<T>, DB_FAILURES> {
	    	mdbx_open_table!(self | table | err, {

			self.cursor(&table).map_err(|err| err.into())
	    	})
	}
}
