//! Implementation of `trait TxRo/TxRw` for `heed`.

use std::cell::RefCell;

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::DbResult,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for heed::RoTxn<'_> {
    fn commit(self) -> DbResult<()> {
        Ok(heed::RoTxn::commit(self)?)
    }
}

//---------------------------------------------------------------------------------------------------- TxRw
impl TxRo<'_> for RefCell<heed::RwTxn<'_>> {
    fn commit(self) -> DbResult<()> {
        TxRw::commit(self)
    }
}

impl TxRw<'_> for RefCell<heed::RwTxn<'_>> {
    fn commit(self) -> DbResult<()> {
        Ok(heed::RwTxn::commit(self.into_inner())?)
    }

    /// This function is infallible.
    fn abort(self) -> DbResult<()> {
        heed::RwTxn::abort(self.into_inner());
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
