//! TODO

use crate::{DatabaseRw, DbResult, RuntimeError, Table};

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
    /// TODO
    pub const fn key(&self) -> &T::Key {
        self.key
    }

    /// TODO
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
