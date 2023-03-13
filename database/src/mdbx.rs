use libmdbx::{RO, RW, DatabaseKind, TransactionKind};

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
	fn put<T: Table>(&self, key: T::Key, value: T::Value) -> Result<(),DB_FAILURES> {
	    todo!()
	}
    
	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(),DB_FAILURES> {
	    todo!()
	}
    
	fn clear<T: Table>(&self, table: T) -> Result<(),DB_FAILURES> {
	    todo!()
	}
}