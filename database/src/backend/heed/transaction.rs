//! Implementation of `trait TxRo/TxRw` for `heed`.

use std::ops::Deref;

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for heed::RoTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.commit().map_err(Into::into)
    }
}

//---------------------------------------------------------------------------------------------------- TxRw
impl TxRo<'_> for heed::RwTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.commit().map_err(Into::into)
    }
}

impl TxRw<'_> for heed::RwTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.commit().map_err(Into::into)
    }

    fn abort(self) {
        self.abort();
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
