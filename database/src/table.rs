//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::pod::Pod;

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

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

    cfg_if::cfg_if! {
        if #[cfg(all(feature = "sanakirja", not(feature = "heed")))] {
            // TODO: fix this sanakirja bound.

            /// Concrete key type.
            type Key: Pod + sanakirja::Storable;

            /// Concrete key type.
            type Value: Pod + sanakirja::Storable;
        } else {
            /// Concrete key type.
            type Key: Pod;

            /// Concrete value type.
            type Value: Pod;
        }
    }
}

// TODO: subkey support. pending on `heed` changes.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
