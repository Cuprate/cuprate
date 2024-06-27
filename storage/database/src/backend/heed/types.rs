//! `heed` type aliases.

//---------------------------------------------------------------------------------------------------- Use
use crate::backend::heed::storable::{KeyHeed, StorableHeed};

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `heed`, usable for reads and writes.
pub(super) type HeedDb<K, V> = heed::Database<KeyHeed<K>, StorableHeed<V>, KeyHeed<K>>;
