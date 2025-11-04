use std::cell::RefCell;
use fjall::{TxPartitionHandle, ReadTransaction, WriteTransaction};
use crate::{DatabaseIter, DatabaseRo, DatabaseRw, DbResult, RuntimeError, Storable, Table};

pub(crate) struct FjallTableRo<'tx> {
    pub(crate) handle: TxPartitionHandle,
    pub(crate) read_tx: &'tx ReadTransaction
}

pub(crate) struct FjallTableRw<'tx, 'env> {
    pub handle: TxPartitionHandle,
    pub read_tx: &'tx RefCell<WriteTransaction<'env>>
}

impl<T: Table> DatabaseIter<T> for FjallTableRo<'_> {
    fn iter(&self) -> DbResult<impl Iterator<Item=DbResult<(T::Key, T::Value)>> + '_> {
        Ok(self.read_tx.iter(&self.handle).map(|item| {
            let item = item?;
            Ok((T::Key::from_bytes(item.0.as_ref()), T::Value::from_bytes(item.1.as_ref())))
        }))
    }

    fn keys(&self) -> DbResult<impl Iterator<Item=DbResult<T::Key>> + '_> {
        Ok(self.read_tx.iter(&self.handle).map(|item| {
            let item = item?;
            Ok(T::Key::from_bytes(item.0.as_ref()))
        }))
    }

    fn values(&self) -> DbResult<impl Iterator<Item=DbResult<T::Value>> + '_> {
        Ok(self.read_tx.iter(&self.handle).map(|item| {
            let item = item?;
            Ok(T::Value::from_bytes(item.1.as_ref()))
        }))
    }
}

unsafe impl<T: Table> DatabaseRo<T> for FjallTableRo<'_> {
    fn get(&self, key: &T::Key) -> DbResult<T::Value> {
        let value = self.read_tx.get(&self.handle, key.as_bytes())?;

        value.map(|s| T::Value::from_bytes(s.as_ref())).ok_or(RuntimeError::KeyNotFound)
    }

    fn len(&self) -> DbResult<u64> {
        Ok(self.read_tx.len(&self.handle)? as u64)
    }

    fn first(&self) -> DbResult<(T::Key, T::Value)> {
        let value = self.read_tx.first_key_value(&self.handle)?;

        value.map(|s| (T::Key::from_bytes(s.0.as_ref()), T::Value::from_bytes(s.1.as_ref()))).ok_or(RuntimeError::KeyNotFound)
    }

    fn last(&self) -> DbResult<(T::Key, T::Value)> {
        let value = self.read_tx.last_key_value(&self.handle)?;

        value.map(|s| (T::Key::from_bytes(s.0.as_ref()), T::Value::from_bytes(s.1.as_ref()))).ok_or(RuntimeError::KeyNotFound)

    }

    fn is_empty(&self) -> DbResult<bool> {
        Ok(self.read_tx.is_empty(&self.handle)?)
    }
}

unsafe impl<T: Table> DatabaseRo<T> for FjallTableRw<'_, '_> {
    fn get(&self, key: &T::Key) -> DbResult<T::Value> {
        let value = self.read_tx.borrow().get(&self.handle, key.as_bytes())?;

        value.map(|s| T::Value::from_bytes(s.as_ref())).ok_or(RuntimeError::KeyNotFound)
    }

    fn len(&self) -> DbResult<u64> {
        Ok(self.read_tx.borrow().len(&self.handle)? as u64)
    }

    fn first(&self) -> DbResult<(T::Key, T::Value)> {
        let value = self.read_tx.borrow().first_key_value(&self.handle)?;

        value.map(|s| (T::Key::from_bytes(s.0.as_ref()), T::Value::from_bytes(s.1.as_ref()))).ok_or(RuntimeError::KeyNotFound)
    }

    fn last(&self) -> DbResult<(T::Key, T::Value)> {
        let value = self.read_tx.borrow().last_key_value(&self.handle)?;

        value.map(|s| (T::Key::from_bytes(s.0.as_ref()), T::Value::from_bytes(s.1.as_ref()))).ok_or(RuntimeError::KeyNotFound)

    }

    fn is_empty(&self) -> DbResult<bool> {
        Ok(self.read_tx.borrow().len(&self.handle)? == 0)
    }
}

impl<T: Table> DatabaseRw<T> for FjallTableRw<'_, '_> {
    fn put(&mut self, key: &T::Key, value: &T::Value, _: bool) -> DbResult<()> {
        self.read_tx.borrow_mut().insert(&self.handle, key.as_bytes(), value.as_bytes());
        Ok(())
    }

    fn delete(&mut self, key: &T::Key) -> DbResult<()> {
        self.read_tx.borrow_mut().remove(&self.handle, key.as_bytes());
        Ok(())
    }

    fn take(&mut self, key: &T::Key) -> DbResult<T::Value> {
        let mut tx = self.read_tx.borrow_mut();
        let value = tx.take(&self.handle, key.as_bytes())?;

        value.map(|s| T::Value::from_bytes(s.as_ref())).ok_or(RuntimeError::KeyNotFound)
    }

    fn pop_first(&mut self) -> DbResult<(T::Key, T::Value)> {
        let mut tx = self.read_tx.borrow_mut();
        let key = tx.first_key_value(&self.handle)?.ok_or(RuntimeError::KeyNotFound)?;

        let value = tx.take(&self.handle, key.0.as_ref())?.ok_or(RuntimeError::KeyNotFound)?;

        Ok((T::Key::from_bytes(key.0.as_ref()), T::Value::from_bytes(value.as_ref())))
    }

    fn pop_last(&mut self) -> DbResult<(T::Key, T::Value)> {
        let mut tx = self.read_tx.borrow_mut();

        let key = tx.last_key_value(&self.handle)?.ok_or(RuntimeError::KeyNotFound)?;

        let value = tx.take(&self.handle, key.0.as_ref())?.ok_or(RuntimeError::KeyNotFound)?;

        Ok((T::Key::from_bytes(key.0.as_ref()), T::Value::from_bytes(value.as_ref())))

    }
}