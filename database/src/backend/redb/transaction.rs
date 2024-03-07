//! Implementation of `trait TxRo/TxRw` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    config::SyncMode,
    env::Env,
    error::RuntimeError,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for redb::ReadTransaction<'_> {
    /// This function is infallible.
    fn commit(self) -> Result<(), RuntimeError> {
        // `redb`'s read transactions cleanup in their `drop()`, there is no `commit()`.
        // https://docs.rs/redb/latest/src/redb/transactions.rs.html#1258-1265
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- TxRw
impl TxRw<'_> for redb::WriteTransaction<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        Ok(self.commit()?)
    }

    fn abort(self) -> Result<(), RuntimeError> {
        Ok(self.abort()?)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
