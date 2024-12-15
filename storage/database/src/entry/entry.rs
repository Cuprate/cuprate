//! TODO

use crate::{
    entry::{OccupiedEntry, VacantEntry},
    DatabaseRw, DbResult, Table,
};

pub enum Entry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    Vacant(VacantEntry<'a, T, D>),
    Occupied(OccupiedEntry<'a, T, D>),
}

impl<'a, T, D> Entry<'a, T, D>
where
    T: Table,
    D: DatabaseRw<T>,
{
    /// TODO
    pub fn or_insert(self, default: &T::Value) -> DbResult<()> {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    /// TODO
    pub fn or_insert_with<F>(self, default: F) -> DbResult<()>
    where
        F: FnOnce() -> &'a T::Value,
    {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    /// TODO
    pub fn or_insert_with_key<F>(self, default: F) -> DbResult<()>
    where
        F: FnOnce(&'a T::Key) -> &'a T::Value,
    {
        match self {
            Self::Occupied(_) => Ok(()),
            Self::Vacant(entry) => {
                let key = entry.key;
                entry.insert(default(key))
            }
        }
    }

    /// TODO
    pub const fn key(&self) -> &T::Key {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }

    /// TODO
    pub const fn value(&self) -> Option<&T::Value> {
        match self {
            Self::Occupied(entry) => Some(entry.value()),
            Self::Vacant(_) => None,
        }
    }

    /// TODO
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

impl<'a, T, D> Entry<'a, T, D>
where
    T: Table,
    <T as Table>::Value: Default,
    D: DatabaseRw<T>,
{
    /// TODO
    pub fn or_default(self) -> &'a mut T::Value {
        todo!()
    }
}
