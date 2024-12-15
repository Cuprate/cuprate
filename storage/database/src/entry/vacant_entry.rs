//! [`VacantEntry`]

use crate::{DatabaseRw, DbResult, RuntimeError, Table};

/// A view into a vacant entry in a [`DatabaseRw`]. It is part of [`crate::entry::Entry`].
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
