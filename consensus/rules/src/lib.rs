cfg_if::cfg_if! {
    // Used in external `tests/`.
    if #[cfg(test)] {
        use proptest as _;
        use proptest_derive as _;
        use tokio as _;
    }
}

use std::time::{SystemTime, UNIX_EPOCH};

pub mod batch_verifier;
pub mod blocks;
mod decomposed_amount;
pub mod genesis;
pub mod hard_forks;
pub mod miner_tx;
pub mod transactions;

pub use decomposed_amount::is_decomposed_amount;
pub use hard_forks::{check_block_version_vote, HFVotes, HFsInfo, HardFork};
pub use transactions::TxVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConsensusError {
    #[error("Block error: {0}")]
    Block(#[from] blocks::BlockError),
    #[error("Transaction error: {0}")]
    Transaction(#[from] transactions::TransactionError),
}

/// Returns the current UNIX timestamp.
pub fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// An internal function that returns an iterator or a parallel iterator if the
/// `rayon` feature is enabled.
#[cfg(feature = "rayon")]
fn try_par_iter<T>(t: T) -> T::Iter
where
    T: rayon::iter::IntoParallelIterator,
{
    t.into_par_iter()
}

/// An internal function that returns an iterator or a parallel iterator if the
/// `rayon` feature is enabled.
#[cfg(not(feature = "rayon"))]
fn try_par_iter<T>(t: T) -> impl Iterator<Item = T::Item>
where
    T: IntoIterator,
{
    t.into_iter()
}
