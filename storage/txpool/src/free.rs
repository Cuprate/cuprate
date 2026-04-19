//! General free functions (related to the tx-pool database).

use crate::types::TransactionBlobHash;

/// Calculate the transaction blob hash.
///
/// This value is supposed to be quick to compute just based of the tx-blob without needing to parse the tx.
///
/// The exact way the hash is calculated is not stable and is subject to change, as such it should not be exposed
/// as a way to interact with Cuprate externally.
pub fn transaction_blob_hash(tx_blob: &[u8]) -> TransactionBlobHash {
    blake3::hash(tx_blob).into()
}
