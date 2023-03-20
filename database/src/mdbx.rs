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


macro_rules! match_open_table {
	($s:ident, $x:block) => {
		match (&$s).open_table(Some(T::TABLE_NAME)) {
			Ok(table) => {
				$x
			}
			Err(err) => Err(err.into())
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
		match_open_table!(self, {
			let mut encoded_key = Vec::new();
			key.consensus_encode(&mut encoded_key);
			match self.get::<Vec<u8>>(&table, &encoded_key) {
				Ok(data) => match data {
					Some(data) => match monero::consensus::deserialize::<T::Value>(&data) {
						Ok(decoded) => Ok(Some(decoded)),
						Err(_) => Err(DB_FAILURES::EncodingError),
					},
					None => Ok(None),
				},
				Err(err) => Err(err.into()),
			}
		})
	}

	fn commit(self) -> Result<(), DB_FAILURES> {
		match self.commit() {
			Ok(res) => {
				if res {
					Ok(())
				} else {
					Err(DB_FAILURES::FailedToCommit)
				}
			},
			Err(err) => Err(err.into()),
		}
	}

	fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DB_FAILURES> {
		match self.open_table(Some(T::TABLE_NAME)) {
			Ok(table) => match self.cursor(&table) {
				Ok(cursor) => Ok(cursor),
				Err(err) => Err(err.into()),
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

	fn put<T: Table>(&self, key: &T::Key, value: &T::Value) -> Result<usize, DB_FAILURES> {
		let mut encoded_key: Vec<u8> = Vec::new();
		match key.consensus_encode(&mut encoded_key) {
			Ok(len_k) => {
				let mut encoded_value: Vec<u8> = Vec::new();
				match value.consensus_encode(&mut encoded_value) {
					Ok(len_v) => match self.open_table(Some(T::TABLE_NAME)) {
						Ok(table) => {
							if let Err(err) = self.put(&table, encoded_key, encoded_value, WriteFlags::empty()) {
								return Err(err.into());
							}
							return Ok(len_k + len_v);
						},
						Err(err) => Err(err.into()),
					},
					Err(err) => Err(DB_FAILURES::EncodingError),
				}
			},
			Err(err) => Err(DB_FAILURES::EncodingError),
		}
	}

	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(), DB_FAILURES> {
		let mut encoded_key: Vec<u8> = Vec::new();
		let mut encoded_value: Vec<u8> = Vec::new();
		let mut ref_ev: Option<&[u8]> = None;
		match key.consensus_encode(&mut encoded_key) {
			Ok(_) => {
				if let Some(value) = value {
					if let Err(err) = value.consensus_encode(&mut encoded_value) {
						return Err(DB_FAILURES::EncodingError);
					} else {
						ref_ev = Some(&encoded_value.as_slice());
					}
				}
				match self.open_table(Some(T::TABLE_NAME)) {
					Ok(table) => {
						if let Err(err) = self.del(&table, encoded_key, ref_ev) {
							return Err(err.into());
						}
						Ok(())
					},
					Err(err) => Err(err.into()),
				}
			},
			Err(err) => Err(DB_FAILURES::EncodingError),
		}
	}

	fn clear<T: Table>(&self) -> Result<(), DB_FAILURES> {
		match self.open_table(Some(T::TABLE_NAME)) {
			Ok(table) => {
				if let Err(err) = self.clear_table(&table) {
					return Err(err.into());
				}
				Ok(())
			},
			Err(err) => Err(err.into()),
		}
	}
}
