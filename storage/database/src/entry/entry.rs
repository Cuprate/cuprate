//! [`Entry`]

use crate::{
    entry::{OccupiedEntry, VacantEntry},
    DatabaseRw, DbResult, RuntimeError, Table,
};

/// A view into a single entry in a [`DatabaseRw`], which may either be vacant or occupied.
///
/// This enum is constructed from the [`DatabaseRw::entry`] method.
pub enum Entry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// A vacant entry; this key did not exist.
    ///
    /// [`crate::Runtime::KeyExists`] will never be returned from this type's operations.
    Vacant(VacantEntry<'a, T, D>),

    /// An occupied entry; this key already exists.
    ///
    /// [`crate::Runtime::KeyNotFound`] will never be returned from this type's operations.
    Occupied(OccupiedEntry<'a, T, D>),
}

impl<'a, T, D> Entry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// TODO
    pub const fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied(_))
    }

    /// TODO
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

    /// Conditionally [`OccupiedEntry::remove`] the value if it already exists.
    ///
    /// This functions does nothing if the entry is [`VacantEntry`].
    pub fn and_remove<F>(self, f: F) -> DbResult<Self>
    where
        F: FnOnce(&T::Value) -> bool,
    {
        Ok(match self {
            Self::Occupied(entry) => {
                if f(&entry.value) {
                    entry.remove()?.0
                } else {
                    Self::Occupied(entry)
                }
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        })
    }

    /// [`OccupiedEntry::update`] the value if it already exists
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
