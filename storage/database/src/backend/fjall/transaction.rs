use std::cell::RefCell;
use fjall::{ReadTransaction, WriteTransaction};
use crate::{DbResult, TxRo, TxRw};

impl TxRo<'_> for ReadTransaction {
    fn commit(self) -> DbResult<()> {
        Ok(())
    }
}

impl TxRo<'_> for RefCell<WriteTransaction<'_>> {
    fn commit(self) -> DbResult<()> {
        Ok(())
    }
}

impl TxRw<'_> for RefCell<WriteTransaction<'_>> {
    fn commit(self) -> DbResult<()> {
        WriteTransaction::commit(self.into_inner())?;
        Ok(())
    }

    fn abort(self) -> DbResult<()> {
        self.into_inner().rollback();
        Ok(())
    }
}