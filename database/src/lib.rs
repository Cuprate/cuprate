// Copyright (C) 2023 Cuprate Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! The cuprate-db crate implement (as its name suggests) the relations between the blockchain/txpool objects and their databases.
//! `lib.rs` contains all the generics, trait and specification for interfaces between blockchain and a backend-agnostic database
//! Every other files in this folder are implementation of these traits/methods to real storage engine.
//!
//! At the moment, the only storage engine available is MDBX.
//! The next storage engine planned is HSE (Heteregeonous Storage Engine) from Micron.
//!
//! For more informations, please consult this docs:

#![deny(unused_attributes)]
#![forbid(unsafe_code)]
#![allow(non_camel_case_types)]
#![deny(clippy::expect_used, clippy::panic)]
#![allow(dead_code, unused_macros)] // temporary

#[cfg(feature = "mdbx")]
pub mod mdbx;
//#[cfg(feature = "hse")]
//pub mod hse;

pub mod encoding;
pub mod interface;
pub mod table;
pub mod types;

const DEFAULT_BLOCKCHAIN_DATABASE_DIRECTORY: &str = "blockchain";
const DEFAULT_TXPOOL_DATABASE_DIRECTORY: &str = "txpool_mem";
const BINCODE_CONFIG: bincode::config::Configuration<
    bincode::config::LittleEndian,
    bincode::config::Fixint,
> = bincode::config::standard().with_fixed_int_encoding();

// ------------------------------------------|      Errors      |------------------------------------------

pub mod error {
	//! ### Error module
	//! This module contains all errors abstraction used by the database crate. By implementing [`From<E>`] to the specific errors of storage engine crates, it let us
	//! handle more easily any type of error that can happen. This module does **NOT** contain interpretation of these errors, as these are defined for Blockchain abstraction. This is another difference
	//! from monerod which interpret these errors directly in its database functions:
	//! ```cpp
	//! /**
	//! * @brief A base class for BlockchainDB exceptions
	//! */
	//! class DB_EXCEPTION : public std::exception
	//! ```
	//! see `blockchain_db/blockchain_db.h` in monerod `src/` folder for more details.

	use thiserror::Error;
	use crate::encoding;

	/// `DBException` is a enum for backend-agnostic, errors that occur during database access. The `From` Trait is implemented through `thiserror` crate and let us easily convert
	/// backend errors into a `DBException`
	#[derive(Error, Debug)]
	pub enum DBException {
	
		// Database errors :
	
		#[cfg(feature = "mdbx")]
		#[error("MDBX failed and returned an error: {0}")]
		MDBX_Error(#[from] libmdbx::Error),

		//  Generic errors :

		#[error("The database failed at an encoding task: {0}")]
		EncodeError(#[from] encoding::Error),

		#[error("Attempting to infer with an object already existent in the database: {0}")]
		AlreadyExist(String),

		#[error("The object have not been found in the database: {0}")]
		NotFound(String),

		#[error("An uncategorized error occured: {0}")]
		Other(&'static str),

	}

}

// ------------------------------------------|      Database      |------------------------------------------

pub mod database {
	//! ### Database module
    //! This module contains the Database abstraction trait. Any key/value storage engine implemented need
    //! to fullfil these associated types and functions, in order to be usable. This module also contains the
    //! Interface struct which is used by the DB Reactor to interact with the database.

    use crate::{database::transaction::{Transaction, WriteTransaction}, error::DBException };
    use std::{ops::Deref, path::PathBuf, sync::Arc};

    /// `Database` Trait implement all the methods necessary to generate transactions as well as execute specific functions. It also implement generic associated types to identify the
    /// different transaction modes (read & write) and it's native errors.
    pub trait Database<'a> {
        type TX: Transaction<'a>;
        type TXMut: WriteTransaction<'a>;
        type Error: Into<DBException>;

        /// Create a transaction from the database
        fn tx(&'a self) -> Result<Self::TX, Self::Error>;

        /// Create a mutable transaction from the database
        fn tx_mut(&'a self) -> Result<Self::TXMut, Self::Error>;

        /// Open a database from the specified path
        fn open(path: PathBuf) -> Result<Self, Self::Error>
        where
            Self: std::marker::Sized;

        /// Check if the database is built.
        fn check_is_correctly_built(&'a self) -> Result<(), Self::Error>;

        /// Build the database
        fn build(&'a self) -> Result<(), Self::Error>;
    }

    /// `Interface` is a struct containing a shared pointer to the database and transaction's to be used for the implemented method of Interface.
    pub struct Interface<'a, D: Database<'a>> {
        pub db: Arc<D>,
        pub tx: Option<<D as Database<'a>>::TXMut>,
    }

    // Convenient implementations for database.
    impl<'thread, D: Database<'thread>> Interface<'thread, D> {
        fn from(db: Arc<D>) -> Result<Self, DBException> {
            Ok(Self { db, tx: None })
        }

        fn open(&'thread mut self) -> Result<(), DBException> {
            let tx = self.db.tx_mut().map_err(Into::into)?;
            self.tx = Some(tx);
            Ok(())
        }
    }

	// Used to easily dereference the WriteTransaction from the interface.
    impl<'thread, D: Database<'thread>> Deref for Interface<'thread, D> {
        type Target = <D as Database<'thread>>::TXMut;

        fn deref(&self) -> &Self::Target {
            return self.tx.as_ref().unwrap();
        }
    }

	pub mod transaction {
		//! #### Transaction sub-module
		//! This sub-module contains the abstractions of Transactional Key/Value database functions.
		//! Any key/value database/storage engine can be implemented easily for Cuprate as long as
		//! these functions or equivalent logic exist for it.
	
		use crate::{
			error::DBException,
			table::{DupTable, Table},
			encoding::{Key, Value, SubKey},
		};
	
		/// A pair of key|value from a table
		pub type Pair<T> = (Key<T>, Value<T>);
	
		/// Abstraction of a read-only cursor, for simple tables
		pub trait Cursor<'t, T: Table> {
			fn first<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException>;
	
			fn get<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException>;
	
			fn last<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException>;
	
			fn next<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException>;
	
			fn prev<const B: bool>(&mut self) -> Result<Option<Pair<T>>, DBException>;
	
			fn set<const B: bool>(&mut self, key: &Key<T>) -> Result<Option<Value<T>>, DBException>;
		}
	
		/// A pair of subkey/value from a duptable
		pub type SubPair<T> = (SubKey<T>, Value<T>);
		pub type FullPair<T> = (Key<T>, SubPair<T>);
	
		/// Abstraction of a read-only cursor with support for duplicated tables. DupCursor inherit Cursor methods as
		/// a duplicated table can be treated as a simple table.
		pub trait DupCursor<'t, T: DupTable>: Cursor<'t, T> {
			fn first_dup<const B: bool>(&mut self) -> Result<Option<SubPair<T>>, DBException>;
	
			fn get_dup<const B: bool>(&mut self, key: &Key<T>, subkey: &SubKey<T>) 
			-> Result<Option<Value<T>>, DBException>;
	
			fn last_dup<const B: bool>(&mut self) -> Result<Option<SubPair<T>>, DBException>;
	
			fn next_dup<const B: bool>(&mut self) -> Result<Option<FullPair<T>>, DBException>;
	
			fn prev_dup<const B: bool>(&mut self) -> Result<Option<FullPair<T>>, DBException>;
		}
	
		// Abstraction of a read-write cursor, for simple tables. WriteCursor inherit Cursor methods.
		pub trait WriteCursor<'t, T: Table>: Cursor<'t, T> {
			fn put_cursor(&mut self, key: &Key<T>, value: &Value<T>) -> Result<(), DBException>;
	
			fn del(&mut self) -> Result<(), DBException>;
		}
	
		// Abstraction of a read-write cursor with support for duplicated tables. DupWriteCursor inherit DupCursor and WriteCursor methods.
		pub trait DupWriteCursor<'t, T: DupTable>: WriteCursor<'t, T> {
			fn put_cursor_dup(
				&mut self,
				key: &Key<T>,
				subkey: &SubKey<T>,
				value: &Value<T>,
			) -> Result<(), DBException>;
	
			/// Delete all data under associated to its key
			fn del_nodup(&mut self) -> Result<(), DBException>;
		}
	
		// Abstraction of a read-only transaction.
		pub trait Transaction<'a>: Send + Sync {
			type Cursor<T: Table>: Cursor<'a, T>;
			type DupCursor<T: DupTable>: DupCursor<'a, T> + Cursor<'a, T>;
	
			fn get<const B: bool, T: Table>(&self, key: &T::Key) -> Result<Option<Value<T>>, DBException>;
	
			fn commit(self) -> Result<(), DBException>;
	
			fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DBException>;
	
			fn cursor_dup<T: DupTable>(&self) -> Result<Self::DupCursor<T>, DBException>;
	
			fn num_entries<T: Table>(&self) -> Result<usize, DBException>;
		}
	
		// Abstraction of a read-write transaction. WriteTransaction inherits Transaction methods.
		pub trait WriteTransaction<'a>: Transaction<'a> {
			type WriteCursor<T: Table>: WriteCursor<'a, T>;
			type DupWriteCursor<T: DupTable>: DupWriteCursor<'a, T> + DupCursor<'a, T>;
	
			fn put<T: Table>(&self, key: &Key<T>, value: &Value<T>) -> Result<(), DBException>;
	
			fn delete<T: Table>(
				&self,
				key: &Key<T>,
				value: &Option<Value<T>>,
			) -> Result<(), DBException>;
	
			fn clear<T: Table>(&self) -> Result<(), DBException>;
	
			fn write_cursor<T: Table>(&self) -> Result<Self::WriteCursor<T>, DBException>;
	
			fn write_cursor_dup<T: DupTable>(&self) -> Result<Self::DupWriteCursor<T>, DBException>;
		}

	}

}


