//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- RoTx
/// Read-only database transaction.
///
/// TODO
pub trait RoTx<'db> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- RwTx
/// Read/write database transaction.
///
/// TODO
pub trait RwTx<'db> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;

    /// TODO
    fn abort(self);
}

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
