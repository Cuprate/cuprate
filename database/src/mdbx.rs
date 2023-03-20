use libmdbx::{RO, RW, DatabaseKind, TransactionKind, WriteFlags, Cursor};
use monero::consensus::Encodable;
use serde::Serialize;

use crate::{
	database::Database,
	error::DB_FAILURES,
	table::Table,
	transaction::{Transaction, WriteTransaction},
};

impl From<libmdbx::Error> for DB_FAILURES {
	fn from(err: libmdbx::Error) -> Self {
		use libmdbx::Error;
		match err {
			Error::Corrupted => DB_FAILURES::Corrupted,
			Error::Panic => DB_FAILURES::Panic,
			Error::KeyExist => DB_FAILURES::KeyAlreadyExist,
			Error::NotFound => DB_FAILURES::KeyNotFound,
			Error::NoData => DB_FAILURES::DataNotFound,
			Error::TooLarge => DB_FAILURES::DataSizeLimit,
			_ => DB_FAILURES::Undefined(0),
		}
	}
}

macro_rules! mdbx_encode_consensus {
    	( $x:ident, $y:ident ) => {
		let mut $y: Vec<u8> = Vec::new();
		if let Err(_) = $x.consensus_encode(&mut $y) {
			return Err(DB_FAILURES::EncodingError);
		}
    	};
}

macro_rules! mdbx_decode_consensus {
    	( $x:ident, $y:ident ) => {
		match monero::consensus::deserialize::<T::Value>(&$x) {
			Ok($y) => Ok(Some($y)), 
			Err(_) => Err(DB_FAILURES::EncodingError),
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

impl<'a, T: Table, R: TransactionKind> crate::transaction::Cursor<'a, T> for Cursor<'a, R> {
    	fn first(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
        	match self.first::<Vec<u8>,Vec<u8>>() {
			Ok(pair) => { 
				match pair {
        				Some(pair) => {
						if let Ok(decoded_key) = monero::consensus::deserialize::<T::Key>(&pair.0) {
							if let Ok(decoded_value) = monero::consensus::deserialize::<T::Value>(&pair.1) {
								return Ok(Some((decoded_key, decoded_value)));
							}
						}
						Err(DB_FAILURES::EncodingError)
					},
        				None => Err(DB_FAILURES::DataNotFound),
    				}
			},
            		Err(err) => Err(err.into()),
        	}
    	}

    	fn get(&mut self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
		match self.get_current::<Vec<u8>,Vec<u8>>() {
    			Ok(pair) =>  {
				match pair {
        				Some(pair) => {
						if let Ok(decoded_key) = monero::consensus::deserialize::<T::Key>(&pair.0) {
							if let Ok(decoded_value) = monero::consensus::deserialize::<T::Value>(&pair.1) {
								return Ok(Some((decoded_key, decoded_value)));
							}
						}
						Err(DB_FAILURES::EncodingError)
					},
        				None => Err(DB_FAILURES::DataNotFound),
    				}
			},
    			Err(err) => Err(err.into()),
		}
    	}

    fn last(&self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
        todo!()
    }

    fn  next(&self) -> Result<Option<(<T as Table>::Key, <T as Table>::Value)>,DB_FAILURES> {
        todo!()
    }

    fn prev(&self) -> Result<Option<(<T as Table>::Key,<T as Table>::Value)>,DB_FAILURES> {
        todo!()
    }

    fn set(&self) -> Result<Option<<T as Table>::Value>,DB_FAILURES> {
        todo!()
    }
}

impl<'a, T: Table> crate::transaction::WriteCursor<'a, T> for Cursor<'a, RW> {
    fn put(&self, key: <T as Table>::Key, value: <T as Table>::Value) -> Result<(),DB_FAILURES> {
        todo!()
    }

    fn del(&self) -> Result<(),DB_FAILURES> {
        todo!()
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
			match self.get::<Vec<u8>>(&table, &encoded_key) {
				Ok(data) => {
					match data {
						Some(data) => mdbx_decode_consensus!(data, decoded),
						None => Ok(None)
					}
				},
				Err(err) => Err(err.into()),
			}
		})
	}

	fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DB_FAILURES> {
		mdbx_open_table!(self | table | err, {
			match self.cursor(&table) {
				Ok(cursor) => Ok(cursor),
				Err(err) => Err(err.into()),
		}})
	}

	fn commit(self) -> Result<(), DB_FAILURES> {
		match self.commit() {
			Ok(res) => {
				if res { Ok(())} 
				else { Err(DB_FAILURES::FailedToCommit) }
			},
			Err(err) => Err(err.into()),
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

			if let Err(err) = self.put(&table, encoded_key, encoded_value, WriteFlags::empty()) {
				return Err(err.into())
			}
			Ok(())
		})
	}

	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self | table | err, {
			mdbx_encode_consensus!(key, encoded_key);
			if let Some(value) = value {
				mdbx_encode_consensus!(value, encoded_value);
				if let Err(err) = self.del(&table, encoded_key, Some(encoded_value.as_slice())) {
					return Err(err.into())
				}
			}
			Ok(())
		})
		
	}

	fn clear<T: Table>(&self) -> Result<(), DB_FAILURES> {
		mdbx_open_table!(self | table | err, {
			if let Err(err) = self.clear_table(&table) {
				return Err(err.into());
			}
			Ok(())
		})
	}
}
