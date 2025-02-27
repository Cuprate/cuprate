//! Entry API for [`DatabaseRw`].
//!
//! This module provides a [`std::collections::btree_map::Entry`]-like API for [`DatabaseRw`].
//!
//! ## Example
//! ```rust
//! use cuprate_database::{
//!     ConcreteEnv,
//!     config::ConfigBuilder,
//!     Env, EnvInner,
//!     DatabaseRo, DatabaseRw, TxRo, TxRw, RuntimeError,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let tmp_dir = tempfile::tempdir()?;
//! # let db_dir = tmp_dir.path().to_owned();
//! # let config = ConfigBuilder::new(db_dir.into()).build();
//! #
//! # let env = ConcreteEnv::open(config)?;
//! #
//! # struct Table;
//! # impl cuprate_database::Table for Table {
//! #     const NAME: &'static str = "table";
//! #     type Key = u8;
//! #     type Value = u64;
//! # }
//! #
//! # let env_inner = env.env_inner();
//! # let tx_rw = env_inner.tx_rw()?;
//! #
//! # env_inner.create_db::<Table>(&tx_rw)?;
//! # let mut table = env_inner.open_db_rw::<Table>(&tx_rw)?;
//! /// The key to use.
//! const KEY: u8 = u8::MAX;
//!
//! /// The update function applied if the value already exists.
//! fn f(value: &mut u64) {
//!     *value += 1;
//! }
//!
//! // No entry exists.
//! assert!(matches!(table.first(), Err(RuntimeError::KeyNotFound)));
//!
//! // Increment the value by `1` or insert a `0` if it doesn't exist.
//! table.entry(&KEY)?.and_update(f)?.or_insert(&0)?;
//! assert_eq!(table.first()?, (KEY, 0));
//! table.entry(&KEY)?.and_update(f)?.or_insert(&0)?;
//! assert_eq!(table.first()?, (KEY, 1));
//!
//! // Conditionally remove the entry.
//! table.entry(&KEY)?.remove_if(|v| *v == 0);
//! assert_eq!(table.first()?, (KEY, 1));
//! table.entry(&KEY)?.remove_if(|v| *v == 1);
//! assert!(matches!(table.first(), Err(RuntimeError::KeyNotFound)));
//! # Ok(()) }
//! ```

use crate::{DatabaseRw, DbResult, RuntimeError, Table};

//---------------------------------------------------------------------------------------------------- Entry
/// A view into a single entry in a [`DatabaseRw`], which may either be a [`VacantEntry`] or [`OccupiedEntry`].
///
/// This enum is constructed from the [`DatabaseRw::entry`] method.
pub enum Entry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// A vacant entry; this key did not exist.
    ///
    /// [`RuntimeError::KeyExists`] will never be returned from this type's operations.
    Vacant(VacantEntry<'a, T, D>),

    /// An occupied entry; this key already exists.
    ///
    /// [`RuntimeError::KeyNotFound`] will never be returned from this type's operations.
    Occupied(OccupiedEntry<'a, T, D>),
}

impl<'a, T, D> Entry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// Returns [`true`] if [`Self::Occupied`].
    pub const fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied(_))
    }

    /// Returns [`true`] if [`Self::Vacant`].
    pub const fn is_vacant(&self) -> bool {
        matches!(self, Self::Vacant(_))
    }

    /// Ensures a value is in the entry by inserting the `default` if empty.
    ///
    /// This only inserts if the entry is [`VacantEntry`].
    pub fn or_insert(self, default: &T::Value) -> DbResult<()> {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the `default` function.
    ///
    /// This only inserts if the entry is [`VacantEntry`].
    pub fn or_insert_with<F>(self, default: F) -> DbResult<()>
    where
        F: FnOnce() -> T::Value,
    {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => entry.insert(&default()),
        }
    }

    /// Same as [`Self::or_insert_with`] but gives access to the key.
    pub fn or_insert_with_key<F>(self, default: F) -> DbResult<()>
    where
        F: FnOnce(&'a T::Key) -> T::Value,
    {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => {
                let key = entry.key;
                entry.insert(&default(key))
            }
        }
    }

    /// Returns a reference to this entry's key.
    pub const fn key(&self) -> &T::Key {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }

    /// Returns a reference to this entry's value (if the entry is [`OccupiedEntry`]).
    ///
    /// # Errors
    /// This returns [`RuntimeError::KeyNotFound`] if the entry is [`VacantEntry`].
    pub const fn value(&self) -> DbResult<&T::Value> {
        match self {
            Self::Occupied(entry) => Ok(entry.value()),
            Self::Vacant(_) => Err(RuntimeError::KeyNotFound),
        }
    }

    /// Take ownership of entry's value (if the entry is [`OccupiedEntry`]).
    ///
    /// # Errors
    /// This returns [`RuntimeError::KeyNotFound`] if the entry is [`VacantEntry`].
    pub fn into_value(self) -> DbResult<T::Value> {
        match self {
            Self::Occupied(entry) => Ok(entry.into_value()),
            Self::Vacant(_) => Err(RuntimeError::KeyNotFound),
        }
    }

    /// Conditionally remove the value if it already exists.
    ///
    /// If `f` returns `true`, the entry will be removed if it exists.
    ///
    /// This functions does nothing if the entry is [`VacantEntry`].
    pub fn remove_if<F>(self, f: F) -> DbResult<Self>
    where
        F: FnOnce(&T::Value) -> bool,
    {
        Ok(match self {
            Self::Occupied(entry) => {
                if f(&entry.value) {
                    Self::Vacant(entry.remove()?.0)
                } else {
                    Self::Occupied(entry)
                }
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        })
    }

    /// Update the value if it already exists.
    ///
    /// This functions does nothing if the entry is [`VacantEntry`].
    pub fn and_update<F>(self, f: F) -> DbResult<Self>
    where
        F: FnOnce(&mut T::Value),
    {
        Ok(match self {
            Self::Occupied(mut entry) => {
                entry.update(f)?;
                Self::Occupied(entry)
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        })
    }
}

impl<T, D> Entry<'_, T, D>
where
    T: Table,
    <T as Table>::Value: Default,
    D: DatabaseRw<T>,
{
    /// Ensures a value is in the entry by inserting a [`Default`] value if empty.
    ///
    /// This only inserts if the entry is [`VacantEntry`].
    pub fn or_default(self) -> DbResult<()> {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => entry.insert(&Default::default()),
        }
    }
}

//---------------------------------------------------------------------------------------------------- VacantEntry
/// A view into a vacant [`Entry`] in a [`DatabaseRw`].
pub struct VacantEntry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    pub(crate) db: &'a mut D,
    pub(crate) key: &'a T::Key,
}

impl<T, D> VacantEntry<'_, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// Gets a reference to the key that is used when [`Self::insert`]ing a value.
    pub const fn key(&self) -> &T::Key {
        self.key
    }

    /// [`DatabaseRw::put`] a new value with [`Self::key`] as the key.
    pub fn insert(self, value: &T::Value) -> DbResult<()> {
        match DatabaseRw::put(self.db, self.key, value) {
            Ok(()) => Ok(()),
            Err(RuntimeError::KeyExists) => {
                unreachable!("this error popping up while VacantEntry exists is a logical error")
            }
            Err(e) => Err(e),
        }
    }
}

//---------------------------------------------------------------------------------------------------- OccupiedEntry
/// A view into an occupied [`Entry`] in a [`DatabaseRw`].
pub struct OccupiedEntry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    pub(crate) db: &'a mut D,
    pub(crate) key: &'a T::Key,
    pub(crate) value: T::Value,
}

impl<'a, T, D> OccupiedEntry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// Gets a reference to the key that is used when [`Self::insert`]ing a value.
    pub const fn key(&self) -> &T::Key {
        self.key
    }

    /// Gets a reference to the current value.
    ///
    /// [`Self::update`] will modify this value.
    pub const fn value(&self) -> &T::Value {
        &self.value
    }

    /// Take ownership of the current value.
    pub fn into_value(self) -> T::Value {
        self.value
    }

    /// Modify the current value and insert it.
    ///
    /// Calling [`Self::value`] after this will return the modified value.
    pub fn update<F>(&mut self, f: F) -> DbResult<()>
    where
        F: FnOnce(&mut T::Value),
    {
        f(&mut self.value);
        DatabaseRw::put(self.db, self.key, &self.value)
    }

    /// Replace the current value with a new value.
    pub fn insert(self, value: &T::Value) -> DbResult<()> {
        DatabaseRw::put(self.db, self.key, value)
    }

    /// Remove this entry.
    ///
    /// The returned values are:
    /// - A [`VacantEntry`]
    /// - The value that was removed
    pub fn remove(self) -> DbResult<(VacantEntry<'a, T, D>, T::Value)> {
        DatabaseRw::delete(self.db, self.key)?;
        Ok((
            VacantEntry {
                db: self.db,
                key: self.key,
            },
            self.value,
        ))
    }
}
