//! `redb` type aliases.

//---------------------------------------------------------------------------------------------------- Types
use crate::{backend::redb::storable::StorableRedb, table::Table};

//---------------------------------------------------------------------------------------------------- Types
/// The concrete type for readable `redb` tables.
pub(super) type RedbTableRo<K, V> = redb::ReadOnlyTable<StorableRedb<K>, StorableRedb<V>>;

/// The concrete type for readable/writable `redb` tables.
pub(super) type RedbTableRw<'tx, K, V> = redb::Table<'tx, StorableRedb<K>, StorableRedb<V>>;
