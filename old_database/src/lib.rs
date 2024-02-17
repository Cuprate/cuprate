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
//! For more information, please consult this docs:

#![deny(unused_attributes)]
#![forbid(unsafe_code)]
#![allow(non_camel_case_types)]
#![deny(clippy::expect_used, clippy::panic)]
#![allow(dead_code, unused_macros)] // temporary

use monero::{util::ringct::RctSig, Block, BlockHeader, Hash};
use std::ops::Range;
use thiserror::Error;

#[cfg(feature = "mdbx")]
pub mod mdbx;
//#[cfg(feature = "hse")]
//pub mod hse;

pub mod encoding;
pub mod error;
pub mod interface;
pub mod table;
pub mod types;

const DEFAULT_BLOCKCHAIN_DATABASE_DIRECTORY: &str = "blockchain";
const DEFAULT_TXPOOL_DATABASE_DIRECTORY: &str = "txpool_mem";
const BINCODE_CONFIG: bincode::config::Configuration<
    bincode::config::LittleEndian,
    bincode::config::Fixint,
> = bincode::config::standard().with_fixed_int_encoding();

// ------------------------------------------|      Database      |------------------------------------------

pub mod database {
    //! This module contains the Database abstraction trait. Any key/value storage engine implemented need
    //! to fulfil these associated types and functions, in order to be usable. This module also contains the
    //! Interface struct which is used by the DB Reactor to interact with the database.

    use crate::{
        error::DB_FAILURES,
        transaction::{Transaction, WriteTransaction},
    };
    use std::{ops::Deref, path::PathBuf, sync::Arc};

    /// `Database` Trait implement all the methods necessary to generate transactions as well as execute specific functions. It also implement generic associated types to identify the
    /// different transaction modes (read & write) and it's native errors.
    pub trait Database<'a> {
        type TX: Transaction<'a>;
        type TXMut: WriteTransaction<'a>;
        type Error: Into<DB_FAILURES>;

        // Create a transaction from the database
        fn tx(&'a self) -> Result<Self::TX, Self::Error>;

        // Create a mutable transaction from the database
        fn tx_mut(&'a self) -> Result<Self::TXMut, Self::Error>;

        // Open a database from the specified path
        fn open(path: PathBuf) -> Result<Self, Self::Error>
        where
            Self: std::marker::Sized;

        // Check if the database is built.
        fn check_all_tables_exist(&'a self) -> Result<(), Self::Error>;

        // Build the database
        fn build(&'a self) -> Result<(), Self::Error>;
    }

    /// `Interface` is a struct containing a shared pointer to the database and transaction's to be used for the implemented method of Interface.
    pub struct Interface<'a, D: Database<'a>> {
        pub db: Arc<D>,
        pub tx: Option<<D as Database<'a>>::TXMut>,
    }

    // Convenient implementations for database
    impl<'service, D: Database<'service>> Interface<'service, D> {
        fn from(db: Arc<D>) -> Result<Self, DB_FAILURES> {
            Ok(Self { db, tx: None })
        }

        fn open(&'service mut self) -> Result<(), DB_FAILURES> {
            let tx = self.db.tx_mut().map_err(Into::into)?;
            self.tx = Some(tx);
            Ok(())
        }
    }

    impl<'service, D: Database<'service>> Deref for Interface<'service, D> {
        type Target = <D as Database<'service>>::TXMut;

        fn deref(&self) -> &Self::Target {
            return self.tx.as_ref().unwrap();
        }
    }
}

// ------------------------------------------|      DatabaseTx     |------------------------------------------

pub mod transaction {
    //! This module contains the abstractions of Transactional Key/Value database functions.
    //! Any key/value database/storage engine can be implemented easily for Cuprate as long as
    //! these functions or equivalent logic exist for it.

    use crate::{
        error::DB_FAILURES,
        table::{DupTable, Table},
    };

    // Abstraction of a read-only cursor, for simple tables
    #[allow(clippy::type_complexity)]
    pub trait Cursor<'t, T: Table> {
        fn first(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES>;

        fn get_cursor(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES>;

        fn last(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES>;

        fn next(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES>;

        fn prev(&mut self) -> Result<Option<(T::Key, T::Value)>, DB_FAILURES>;

        fn set(&mut self, key: &T::Key) -> Result<Option<T::Value>, DB_FAILURES>;
    }

    // Abstraction of a read-only cursor with support for duplicated tables. DupCursor inherit Cursor methods as
    // a duplicated table can be treated as a simple table.
    #[allow(clippy::type_complexity)]
    pub trait DupCursor<'t, T: DupTable>: Cursor<'t, T> {
        fn first_dup(&mut self) -> Result<Option<(T::SubKey, T::Value)>, DB_FAILURES>;

        fn get_dup(
            &mut self,
            key: &T::Key,
            subkey: &T::SubKey,
        ) -> Result<Option<T::Value>, DB_FAILURES>;

        fn last_dup(&mut self) -> Result<Option<(T::SubKey, T::Value)>, DB_FAILURES>;

        fn next_dup(&mut self) -> Result<Option<(T::Key, (T::SubKey, T::Value))>, DB_FAILURES>;

        fn prev_dup(&mut self) -> Result<Option<(T::Key, (T::SubKey, T::Value))>, DB_FAILURES>;
    }

    // Abstraction of a read-write cursor, for simple tables. WriteCursor inherit Cursor methods.
    pub trait WriteCursor<'t, T: Table>: Cursor<'t, T> {
        fn put_cursor(&mut self, key: &T::Key, value: &T::Value) -> Result<(), DB_FAILURES>;

        fn del(&mut self) -> Result<(), DB_FAILURES>;
    }

    // Abstraction of a read-write cursor with support for duplicated tables. DupWriteCursor inherit DupCursor and WriteCursor methods.
    pub trait DupWriteCursor<'t, T: DupTable>: WriteCursor<'t, T> {
        fn put_cursor_dup(
            &mut self,
            key: &T::Key,
            subkey: &T::SubKey,
            value: &T::Value,
        ) -> Result<(), DB_FAILURES>;

        /// Delete all data under associated to its key
        fn del_nodup(&mut self) -> Result<(), DB_FAILURES>;
    }

    // Abstraction of a read-only transaction.
    pub trait Transaction<'a>: Send + Sync {
        type Cursor<T: Table>: Cursor<'a, T>;
        type DupCursor<T: DupTable>: DupCursor<'a, T> + Cursor<'a, T>;

        fn get<T: Table>(&self, key: &T::Key) -> Result<Option<T::Value>, DB_FAILURES>;

        fn commit(self) -> Result<(), DB_FAILURES>;

        fn cursor<T: Table>(&self) -> Result<Self::Cursor<T>, DB_FAILURES>;

        fn cursor_dup<T: DupTable>(&self) -> Result<Self::DupCursor<T>, DB_FAILURES>;

        fn num_entries<T: Table>(&self) -> Result<usize, DB_FAILURES>;
    }

    // Abstraction of a read-write transaction. WriteTransaction inherits Transaction methods.
    pub trait WriteTransaction<'a>: Transaction<'a> {
        type WriteCursor<T: Table>: WriteCursor<'a, T>;
        type DupWriteCursor<T: DupTable>: DupWriteCursor<'a, T> + DupCursor<'a, T>;

        fn put<T: Table>(&self, key: &T::Key, value: &T::Value) -> Result<(), DB_FAILURES>;

        fn delete<T: Table>(
            &self,
            key: &T::Key,
            value: &Option<T::Value>,
        ) -> Result<(), DB_FAILURES>;

        fn clear<T: Table>(&self) -> Result<(), DB_FAILURES>;

        fn write_cursor<T: Table>(&self) -> Result<Self::WriteCursor<T>, DB_FAILURES>;

        fn write_cursor_dup<T: DupTable>(&self) -> Result<Self::DupWriteCursor<T>, DB_FAILURES>;
    }
}
