//! TODO

use crate::{DatabaseRw, DbResult, Table};

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
    /// TODO
    pub const fn key(&self) -> &T::Key {
        self.key
    }

    /// TODO
    pub const fn value(&self) -> &T::Value {
        &self.value
    }

    /// TODO
    pub fn update<F>(&mut self, f: F) -> DbResult<()>
    where
        F: FnOnce(&mut T::Value),
    {
        f(&mut self.value);
        DatabaseRw::put(self.db, self.key, &self.value)
    }

    /// TODO
    pub fn insert(&mut self, value: &T::Value) -> DbResult<()> {
        DatabaseRw::put(self.db, self.key, value)
    }

    /// TODO
    pub fn remove(self) -> DbResult<T::Value> {
        DatabaseRw::delete(self.db, self.key).map(|()| Ok(self.value))?
    }

    /// TODO
    pub const fn get(&self) -> &T::Value {
        &self.value
    }

    /// TODO
    pub fn into_value(self) -> T::Value {
        self.value
    }
}
