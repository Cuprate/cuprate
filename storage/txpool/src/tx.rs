//! Transaction metadata.

/// Data about a transaction in the pool.
///
/// Used in [`TxpoolReadResponse::Backlog`](crate::service::interface::TxpoolReadResponse::Backlog).
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TxEntry {
    /// The transaction's weight.
    pub weight: u64,
    /// The transaction's fee.
    pub fee: u64,
    /// How long the transaction has been in the pool.
    pub time_in_pool: std::time::Duration,
}
