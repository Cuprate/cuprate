//! `heed` type aliases.

//---------------------------------------------------------------------------------------------------- Use
use crate::backend::heed::storable::StorableHeed;

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `heed`, usable for reads and writes.
pub(super) type HeedDb<K, V> = heed::Database<StorableHeed<K>, StorableHeed<V>>;
