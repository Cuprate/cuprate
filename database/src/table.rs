//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Table
/// TODO
///
/// Database table metadata.
///
/// Purely compile time information for database tables.
pub trait Table {
    /// Name of the database table.
    const NAME: &'static str;

    /// TODO: must be (de)serializable into bytes.
    type Key;

    /// TODO: must be (de)serializable into bytes.
    type Value;
}

// TODO: subkey support. pending on `heed` changes.

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
