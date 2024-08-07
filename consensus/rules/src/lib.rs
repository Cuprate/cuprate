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

/// Checks that a point is canonically encoded.
///
/// <https://github.com/dalek-cryptography/curve25519-dalek/issues/380>
fn check_point_canonically_encoded(point: &curve25519_dalek::edwards::CompressedEdwardsY) -> bool {
    let bytes = point.as_bytes();

    point
        .decompress()
        // Ban points which are either unreduced or -0
        .filter(|point| point.compress().as_bytes() == bytes)
        .is_some()
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
fn try_par_iter<T>(t: T) -> impl std::iter::Iterator<Item = T::Item>
where
    T: std::iter::IntoIterator,
{
    t.into_iter()
}
