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

    /// Concrete key type.
    #[cfg(feature = "heed")]
    type Key: Pod;

    /// Concrete key type.
    #[cfg(feature = "sanakirja")] // TODO: fix this bound.
    type Key: Pod + sanakirja::Storable;

    /// Concrete value type.
    #[cfg(feature = "heed")]
    type Value: Pod;

    /// Concrete key type.
    #[cfg(feature = "sanakirja")] // TODO: fix this bound.
    type Value: Pod + sanakirja::Storable;
}

// TODO: subkey support. pending on `heed` changes.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
