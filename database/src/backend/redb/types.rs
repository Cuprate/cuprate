//! `redb` type aliases.

//---------------------------------------------------------------------------------------------------- Types
// TODO: replace `()` with a byte container.

/// The concrete type for readable `redb` tables.
pub(super) type RedbTableRo<'env> = redb::ReadOnlyTable<'env, (), ()>;

/// The concrete type for readable/writable `redb` tables.
pub(super) type RedbTableRw<'env, 'tx> = redb::Table<'env, 'tx, (), ()>;
