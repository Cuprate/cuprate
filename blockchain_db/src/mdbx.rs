use libmdbx::RO;

use crate::database::{Database, Transaction, DB_FAILURES};

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
	E: libmdbx::DatabaseKind 
{
	fn tx<T: Transaction<'a>>(&self, ro: bool) -> Result<T, DB_FAILURES> {
		match ro {
			true => { 
				return self.begin_ro_txn()
			}
			false => {
				return self.begin_rw_txn()
			}
		}
	}
}

// Yes it doesn't work
impl Transaction<'_> for libmdbx::Transaction<'_,RO,libmdbx::Error> {

}