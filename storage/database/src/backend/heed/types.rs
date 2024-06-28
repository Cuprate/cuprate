//! `heed` type aliases.

//---------------------------------------------------------------------------------------------------- Use
use crate::backend::heed::storable::StorableHeed;

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `heed`, usable for reads and writes.
//
//                                         Key type        Value type        Key comparison implementor
//                                            v                v                v
pub(super) type HeedDb<K, V> = heed::Database<StorableHeed<K>, StorableHeed<V>, StorableHeed<K>>;
