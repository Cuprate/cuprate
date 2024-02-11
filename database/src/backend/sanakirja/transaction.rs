//! Implementation of `trait RoTx/RwTx` for `sanakirja`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    transaction::{RoTx, RwTx},
};

//---------------------------------------------------------------------------------------------------- RoTx
impl RoTx<'_> for sanakirja::Txn<&'_ sanakirja::Env> {
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- RwTx
impl RwTx<'_> for sanakirja::MutTxn<&'_ sanakirja::Env, ()> {
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
