//! Conversion from `redb`'s errors -> `cuprate_database`'s errors.
//!
//! HACK: There's a lot of `_ =>` usage here because
//! `redb`'s errors are `#[non_exhaustive]`...

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    constants::DATABASE_CORRUPT_MSG,
    error::{InitError, RuntimeError},
};

//---------------------------------------------------------------------------------------------------- InitError
impl From<redb::DatabaseError> for InitError {
    /// Created by `redb` in:
    /// - [`redb::Database::open`](https://docs.rs/redb/1.5.0/redb/struct.Database.html#method.open).
    fn from(error: redb::DatabaseError) -> Self {
        use redb::DatabaseError as E;
        use redb::StorageError as E2;

        // Reference of all possible errors `redb` will return
        // upon using `redb::Database::open`:
        // <https://docs.rs/redb/1.5.0/src/redb/db.rs.html#908-923>
        match error {
            E::RepairAborted => Self::Corrupt,
            E::UpgradeRequired(_) => Self::InvalidVersion,
            E::Storage(s_error) => match s_error {
                E2::Io(e) => Self::Io(e),
                E2::Corrupted(_) => Self::Corrupt,

                // HACK: Handle new errors as `redb` adds them.
                _ => Self::Unknown(Box::new(s_error)),
            },

            // HACK: Handle new errors as `redb` adds them.
            _ => Self::Unknown(Box::new(error)),
        }
    }
}

impl From<redb::StorageError> for InitError {
    /// Created by `redb` in:
    /// - [`redb::Database::open`](https://docs.rs/redb/1.5.0/redb/struct.Database.html#method.check_integrity)
    fn from(error: redb::StorageError) -> Self {
        use redb::StorageError as E;

        match error {
            E::Io(e) => Self::Io(e),
            E::Corrupted(_) => Self::Corrupt,
            // HACK: Handle new errors as `redb` adds them.
            _ => Self::Unknown(Box::new(error)),
        }
    }
}

impl From<redb::TransactionError> for InitError {
    /// Created by `redb` in:
    /// - [`redb::Database::begin_write`](https://docs.rs/redb/1.5.0/redb/struct.Database.html#method.begin_write)
    fn from(error: redb::TransactionError) -> Self {
        match error {
            redb::TransactionError::Storage(error) => error.into(),
            // HACK: Handle new errors as `redb` adds them.
            _ => Self::Unknown(Box::new(error)),
        }
    }
}

impl From<redb::TableError> for InitError {
    /// Created by `redb` in:
    /// - [`redb::WriteTransaction::open_table`](https://docs.rs/redb/1.5.0/redb/struct.WriteTransaction.html#method.open_table)
    fn from(error: redb::TableError) -> Self {
        use redb::TableError as E;

        match error {
            E::Storage(error) => error.into(),
            // HACK: Handle new errors as `redb` adds them.
            _ => Self::Unknown(Box::new(error)),
        }
    }
}

impl From<redb::CommitError> for InitError {
    /// Created by `redb` in:
    /// - [`redb::WriteTransaction::commit`](https://docs.rs/redb/1.5.0/redb/struct.WriteTransaction.html#method.commit)
    fn from(error: redb::CommitError) -> Self {
        match error {
            redb::CommitError::Storage(error) => error.into(),
            // HACK: Handle new errors as `redb` adds them.
            _ => Self::Unknown(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- RuntimeError
#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<redb::TransactionError> for RuntimeError {
    /// Created by `redb` in:
    /// - [`redb::Database::begin_write`](https://docs.rs/redb/1.5.0/redb/struct.Database.html#method.begin_write)
    /// - [`redb::Database::begin_read`](https://docs.rs/redb/1.5.0/redb/struct.Database.html#method.begin_read)
    fn from(error: redb::TransactionError) -> Self {
        match error {
            redb::TransactionError::Storage(error) => error.into(),

            // HACK: Handle new errors as `redb` adds them.
            _ => unreachable!(),
        }
    }
}

#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<redb::CommitError> for RuntimeError {
    /// Created by `redb` in:
    /// - [`redb::WriteTransaction::commit`](https://docs.rs/redb/1.5.0/redb/struct.WriteTransaction.html#method.commit)
    fn from(error: redb::CommitError) -> Self {
        match error {
            redb::CommitError::Storage(error) => error.into(),

            // HACK: Handle new errors as `redb` adds them.
            _ => unreachable!(),
        }
    }
}

#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<redb::TableError> for RuntimeError {
    /// Created by `redb` in:
    /// - [`redb::WriteTransaction::open_table`](https://docs.rs/redb/1.5.0/redb/struct.WriteTransaction.html#method.open_table)
    /// - [`redb::ReadTransaction::open_table`](https://docs.rs/redb/1.5.0/redb/struct.ReadTransaction.html#method.open_table)
    fn from(error: redb::TableError) -> Self {
        use redb::TableError as E;

        match error {
            E::Storage(error) => error.into(),

            E::TableDoesNotExist(_) => Self::TableNotFound,

            // Only if we write incorrect code.
            E::TableTypeMismatch { .. }
            | E::TableIsMultimap(_)
            | E::TableIsNotMultimap(_)
            | E::TypeDefinitionChanged { .. }
            | E::TableAlreadyOpen(..) => panic!("fix the database code! {error:#?}"),

            // HACK: Handle new errors as `redb` adds them.
            _ => unreachable!(),
        }
    }
}

#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<redb::StorageError> for RuntimeError {
    /// Created by `redb` in:
    /// - [`redb::Table`](https://docs.rs/redb/1.5.0/redb/struct.Table.html) functions
    /// - [`redb::ReadOnlyTable`](https://docs.rs/redb/1.5.0/redb/struct.ReadOnlyTable.html) functions
    fn from(error: redb::StorageError) -> Self {
        use redb::StorageError as E;

        match error {
            E::Io(e) => Self::Io(e),
            E::Corrupted(s) => panic!("{s:#?}\n{DATABASE_CORRUPT_MSG}"),
            E::ValueTooLarge(s) => panic!("fix the database code! {s:#?}"),
            E::LockPoisoned(s) => panic!("{s:#?}"),

            // HACK: Handle new errors as `redb` adds them.
            _ => unreachable!(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
