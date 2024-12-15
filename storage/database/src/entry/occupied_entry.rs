//! [`OccupiedEntry`]

use crate::{DatabaseRw, DbResult, Table};

/// A view into an occupied entry in a [`DatabaseRw`]. It is part of [`crate::entry::Entry`].
pub struct OccupiedEntry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    pub(crate) db: &'a mut D,
    pub(crate) key: &'a T::Key,
    pub(crate) value: T::Value,
}

impl<T, D> OccupiedEntry<'_, T, D>
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
    pub fn remove(self) -> DbResult<T::Value> {
        DatabaseRw::delete(self.db, self.key).map(|()| Ok(self.value))?
    }
}
