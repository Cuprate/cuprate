//! A Monero [`transaction::Transaction`] that remembers its hash.

use std::{
    io::{self, Read},
    ops::Deref,
    sync::OnceLock,
};

use monero_oxide::transaction;

/// A Monero [`transaction::Transaction`] that remembers its hash, calculating it on first use if
/// not given.
#[derive(Clone, Debug)]
pub struct Transaction {
    /// The transaction.
    tx: transaction::Transaction,
    /// The hash of [`Self::tx`].
    hash: OnceLock<[u8; 32]>,
}

impl Transaction {
    /// Wrap a [`transaction::Transaction`].
    pub const fn new(tx: transaction::Transaction) -> Self {
        Self {
            tx,
            hash: OnceLock::new(),
        }
    }

    /// Wrap a [`transaction::Transaction`] whose hash is already known.
    ///
    /// `hash` must be the transaction's hash.
    pub fn with_hash(tx: transaction::Transaction, hash: [u8; 32]) -> Self {
        debug_assert_eq!(tx.hash(), hash);

        Self {
            tx,
            hash: OnceLock::from(hash),
        }
    }

    /// Read a [`transaction::Transaction`].
    ///
    /// # Errors
    /// This errors if the transaction is malformed.
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self::new(transaction::Transaction::read(r)?))
    }

    /// The transaction's hash.
    pub fn hash(&self) -> [u8; 32] {
        *self.hash.get_or_init(|| self.tx.hash())
    }

    /// Unwrap the [`transaction::Transaction`].
    pub fn into_inner(self) -> transaction::Transaction {
        self.tx
    }
}

impl Deref for Transaction {
    type Target = transaction::Transaction;

    fn deref(&self) -> &transaction::Transaction {
        &self.tx
    }
}
