use libmdbx::{RO, RW, DatabaseKind};

use crate::database::{Database, Transaction, DB_FAILURES, Table};


impl From<libmdbx::Error> for DB_FAILURES {
	fn from(err: libmdbx::Error) -> Self {
		use libmdbx::Error;
	    	match err {
			Error::Corrupted 	=> { DB_FAILURES::Corrupted }
			Error::Panic		      => { DB_FAILURES::Panic }
			_ 				 	 => { DB_FAILURES::Undefined(0) }
	    	}
	}
}

impl<'a, E> Database<'a> for libmdbx::Database<E> 
where
	E: DatabaseKind,
{
	type TX = libmdbx::Transaction<'a,RO,E>;
	type TXMut = libmdbx::Transaction<'a,RW,E>;

	fn tx(&'a self, ro: bool) -> Result<Self::TX, DB_FAILURES> {
		match ro {
			true => { 
				match self.begin_ro_txn() {
					Ok(tx) => {
						Ok(tx)
					}
					Err(_) => { todo!() }
				}
			}
			false => {
				todo!();
			}
		}
	}
}


// Yes it doesn't work
impl<E> Transaction<'_> for libmdbx::Transaction<'_,RO,E> 
where
	E: DatabaseKind
{
    fn get<T: Table>(&self, key: T::Key) -> Result<Option<T::Value>, DB_FAILURES> {
        todo!()
    }

    fn put<T: Table>(&self, key: T::Key, value: T::Value) -> Result<(),DB_FAILURES> {
        todo!()
    }

    fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(),DB_FAILURES> {
        todo!()
    }

    fn clear<T: Table>(&self, table: T) -> Result<(),DB_FAILURES> {
        todo!()
    }

    fn commit(self) -> Result<(), DB_FAILURES> {
        todo!()
    }
}

impl<E> Transaction<'_> for libmdbx::Transaction<'_,RW,E> 
where
	E: DatabaseKind
{
	fn get<T: Table>(&self, key: T::Key) -> Result<Option<T::Value>, DB_FAILURES> {
	    todo!()
	}
    
	fn put<T: Table>(&self, key: T::Key, value: T::Value) -> Result<(),DB_FAILURES> {
	    todo!()
	}
    
	fn delete<T: Table>(&self, key: T::Key, value: Option<T::Value>) -> Result<(),DB_FAILURES> {
	    todo!()
	}
    
	fn clear<T: Table>(&self, table: T) -> Result<(),DB_FAILURES> {
	    todo!()
	}
    
	fn commit(self) -> Result<(), DB_FAILURES> {
	    todo!()
	}
}