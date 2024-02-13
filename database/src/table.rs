//! Database table abstraction; `trait Table`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{key::Key, pod::Pod};

//---------------------------------------------------------------------------------------------------- Table
/// Database table metadata.
///
/// Purely compile time information for database tables.
/// Not really an accurate name for `K/V` database but
/// this represents the metadata of a `K/V` storing object.
pub trait Table {
    // TODO:
    //
    // Add K/V comparison `type`s that define
    // how this table will be stored.
    //
    // type KeyComparator: fn(&Self::Key, &Self::Key) -> Ordering;
    // type ValueComparator: fn(&Self::Value, &Self::Value) -> Ordering;

    /// Name of the database table.
    const NAME: &'static str;

    /// Whether the table's values are all the same size or not.
    const CONSTANT_SIZE: bool;

    /// Primary key type.
    type Key: Key;

    // TODO: fix this sanakirja bound.
    cfg_if::cfg_if! {
        if #[cfg(all(feature = "sanakirja", not(feature = "heed")))] {
            /// Value type.
            type Value: Pod + sanakirja::Storable;
        } else {
            /// Value type.
            type Value: Pod;
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
