//! TODO

//---------------------------------------------------------------------------------------------------- Import
// use crate::error::Error;

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Table
/// TODO
///
/// Database table.
pub trait Table {
    /// Name of the database table.
    const NAME: &'static str;

    /// QUESTION:
    /// Should the (de)serialize trait just be `borsh`?

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
