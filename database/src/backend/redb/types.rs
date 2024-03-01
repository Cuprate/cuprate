//! `redb` type aliases.

//---------------------------------------------------------------------------------------------------- Types
use crate::{
    backend::redb::storable::{StorableRedbKey, StorableRedbValue},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Types
/// The concrete type for readable `redb` tables.
pub(super) type RedbTableRo<'env, K, V> =
    redb::ReadOnlyTable<'env, StorableRedbKey<K>, StorableRedbValue<V>>;

/// The concrete type for readable/writable `redb` tables.
pub(super) type RedbTableRw<'env, 'tx, K, V> =
    redb::Table<'env, 'tx, StorableRedbKey<K>, StorableRedbValue<V>>;
