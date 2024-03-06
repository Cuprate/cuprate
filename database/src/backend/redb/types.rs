//! `redb` type aliases.

//---------------------------------------------------------------------------------------------------- Types
use crate::{backend::redb::storable::StorableRedb, table::Table};

//---------------------------------------------------------------------------------------------------- Types
/// The concrete type for readable `redb` tables.
pub(super) type RedbTableRo<'env, K, V> =
    redb::ReadOnlyTable<'env, StorableRedb<K>, StorableRedb<V>>;

/// The concrete type for readable/writable `redb` tables.
pub(super) type RedbTableRw<'env, 'tx, K, V> =
    redb::Table<'env, 'tx, StorableRedb<K>, StorableRedb<V>>;
