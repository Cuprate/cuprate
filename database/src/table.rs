//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::pod::Pod;

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Table
/// TODO
///
/// Database table metadata.
///
/// Purely compile time information for database tables.
/// Not really an accurate name for `K/V` database but
/// this represents the metadata of a `K/V` storing object.
pub trait Table {
    /// Name of the database table.
    const NAME: &'static str;

    /// Concrete key type.
    type Key: Pod;

    /// Concrete value type.
    type Value: Pod;
}

// TODO: subkey support. pending on `heed` changes.

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
