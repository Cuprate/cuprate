use libmdbx::{RO, RW, DatabaseKind, TransactionKind, WriteFlags};
use monero::consensus::Encodable;

use crate::{database::{Database},error::DB_FAILURES,table::Table,transaction::{Transaction, WriteTransaction}};


impl From<libmdbx::Error> for DB_FAILURES {
	fn from(err: libmdbx::Error) -> Self {
		use libmdbx::Error;
	    	match err {
			Error::Corrupted => { DB_FAILURES::Corrupted }
			Error::Panic => { DB_FAILURES::Panic }
			Error::KeyExist => { DB_FAILURES::KeyAlreadyExist }
			Error::NotFound => { DB_FAILURES::KeyNotFound }
			Error::NoData => { DB_FAILURES::DataNotFound }
			Error::TooLarge => { DB_FAILURES::DataSizeLimit }
			_ => { DB_FAILURES::Undefined(0) }
	    	}
	}
}

impl<'a, E> Database<'a> for libmdbx::Database<E> 
where
	E: DatabaseKind,
{
	type TX = libmdbx::Transaction<'a,RO,E>;
	type TXMut = libmdbx::Transaction<'a,RW,E>;
	type Error = libmdbx::Error;

	fn tx(&'a self) -> Result<Self::TX, Self::Error> {
		self.begin_ro_txn()
	}

	fn tx_mut(&'a self) -> Result<Self::TXMut, Self::Error> {
		self.begin_rw_txn()
	}
}

// Yes it doesn't work
impl<E, R: TransactionKind> Transaction<'_> for libmdbx::Transaction<'_,R,E> 
where
	E: DatabaseKind
{
    fn get<T: Table>(&self, key: T::Key) -> Result<Option<T::Value>, DB_FAILURES> {
        todo!()
    }

    fn commit(self) -> Result<(), DB_FAILURES> {
        todo!()
    }
}

impl<E> WriteTransaction<'_> for libmdbx::Transaction<'_,RW,E> 
where
	E: DatabaseKind
{
	fn put<T: Table>(&self, key: &T::Key, value: &T::Value) -> Result<usize,DB_FAILURES> {

	    	let mut encoded_key: Vec<u8> = Vec::new();
	    	match key.consensus_encode(&mut encoded_key) {
			Ok(len_k) => {
				let mut encoded_value: Vec<u8> = Vec::new();
				match value.consensus_encode(&mut encoded_value) {
					Ok(len_v) => {
						match self.open_table(Some(T::TABLE_NAME)) {
							Ok(table) => {
								if let Err(err) = self.put(&table, encoded_key, encoded_value, WriteFlags::empty()) {
									return Err(err.into());
								}
								return Ok(len_k+len_v);
							}
							Err(err) => { Err(err.into()) }
						}
					},
					Err(err) => { Err(DB_FAILURES::EncodingError) }
				}
			},
			Err(err) => { Err(DB_FAILURES::EncodingError) }
		}
	}
    
	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(),DB_FAILURES> {
	   	let mut encoded_key: Vec<u8> = Vec::new();
		let mut encoded_value: Vec<u8> = Vec::new();
		let mut ref_ev: Option<&[u8]> = None;
	   	match key.consensus_encode(&mut encoded_key) {
			Ok(_) => {
				if let Some(value) = value {
					if let Err(err) =  value.consensus_encode(&mut encoded_value) {
						return Err(DB_FAILURES::EncodingError)
					} else {
						ref_ev = Some(&encoded_value.as_slice());
					}
				}
				match self.open_table(Some(T::TABLE_NAME)) {
					Ok(table) => {
						if let Err(err) = self.del(&table, encoded_key, ref_ev) {
							return Err(err.into())
						}
						Ok(())
					}
					Err(err) => { Err(err.into()) }
				}
			}
			Err(err) => { Err(DB_FAILURES::EncodingError) }
	   	}
	}
    
	fn clear<T: Table>(&self) -> Result<(),DB_FAILURES> {
	    match self.open_table(Some(T::TABLE_NAME)) {
		Ok(table) => {
			if let Err(err) = self.clear_table(&table) {
				return Err(err.into())
			}
			Ok(())
		}
		Err(err) => { Err(err.into()) }
	    }
	}
}