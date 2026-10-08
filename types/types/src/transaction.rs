//! A Monero [`transaction::Transaction`] with its hash.

use std::{
    io::{self, Read},
    ops::Deref,
};

use monero_oxide::transaction;

/// A Monero [`transaction::Transaction`] with its hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    /// The transaction.
    tx: transaction::Transaction,
    /// The hash of [`Self::tx`].
    hash: [u8; 32],
}

impl Transaction {
    /// Wrap a [`transaction::Transaction`], calculating its hash.
    pub fn new(tx: transaction::Transaction) -> Self {
        Self {
            hash: tx.hash(),
            tx,
        }
    }

    /// Wrap a [`transaction::Transaction`] whose hash is already known.
    ///
    /// `hash` must be the transaction's hash.
    pub fn with_hash(tx: transaction::Transaction, hash: [u8; 32]) -> Self {
        debug_assert_eq!(tx.hash(), hash);

        Self { tx, hash }
    }

    /// Read a [`transaction::Transaction`].
    ///
    /// # Errors
    /// This errors if the transaction is malformed.
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        transaction::Transaction::read(r).map(Self::new)
    }

    /// The transaction's hash.
    pub const fn hash(&self) -> [u8; 32] {
        self.hash
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
