//! Transaction metadata.

/// Data about a transaction in the pool.
///
/// Used in [`TxpoolReadResponse::Backlog`](crate::service::interface::TxpoolReadResponse::Backlog).
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TxEntry {
    /// The transaction's ID (hash).
    pub id: [u8; 32],
    /// The transaction's weight.
    pub weight: usize,
    /// The transaction's fee.
    pub fee: u64,
    /// If the tx is in the private pool.
    pub private: bool,
    /// The UNIX timestamp when the transaction was received.
    pub received_at: u64,
}
