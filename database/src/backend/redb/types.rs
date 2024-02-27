//! `redb` type aliases.

//---------------------------------------------------------------------------------------------------- Types
// TODO: replace `()` with a byte container.

/// The concrete type for readable `redb` tables.
pub(super) type RedbTableRo<'a> = redb::ReadOnlyTable<'a, (), ()>;

/// The concrete type for readable/writable `redb` tables.
pub(super) type RedbTableRw<'db, 'tx> = redb::Table<'db, 'tx, (), ()>;
