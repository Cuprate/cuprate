//! Implementation of `trait TxRo/TxRw` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for redb::ReadTransaction<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- TxRw
impl TxRw<'_> for redb::WriteTransaction<'_> {
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
