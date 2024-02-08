//! Concrete transaction types.
//!
//! These transactions are a combination of typical
//! "transaction" objects alongside an actual `K/V` table.
//!
//! This is done so callers don't need to
//! juggle around tables/transactions, they just:
//!
//! 1. Get a K/V table from the `Database` (1 single database)
//! 2. Do whatever they need to do (`get()`, `put()`, etc)

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- RoTx
impl RoTx<'_> for heed::RoTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- RwTx
impl RwTx<'_> for heed::RwTxn<'_> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }

    /// TODO
    fn abort(self) {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
