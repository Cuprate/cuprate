mod decomposed_amount;
mod hard_forks;
mod miner_tx;
mod signatures;
mod transactions;

pub use decomposed_amount::is_decomposed_amount;
pub use hard_forks::{HFVotes, HFsInfo, HardFork};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TxVersion {
    RingSignatures,
    RingCT,
}

impl TxVersion {
    pub fn from_raw(version: u64) -> Option<TxVersion> {
        Some(match version {
            1 => TxVersion::RingSignatures,
            2 => TxVersion::RingCT,
            _ => return None,
        })
    }
}

/// Checks that a point is canonical.
///
/// https://github.com/dalek-cryptography/curve25519-dalek/issues/380
fn check_point(point: &curve25519_dalek::edwards::CompressedEdwardsY) -> bool {
    let bytes = point.as_bytes();

    point
        .decompress()
        // Ban points which are either unreduced or -0
        .filter(|point| point.compress().as_bytes() == bytes)
        .is_some()
}

#[cfg(feature = "rayon")]
fn try_par_iter<T>(t: T) -> T::Iter
where
    T: rayon::iter::IntoParallelIterator,
{
    t.into_par_iter()
}

#[cfg(not(feature = "rayon"))]
fn try_par_iter<T>(t: T) -> impl std::iter::Iterator<Item = T::Item>
where
    T: std::iter::IntoIterator,
{
    t.into_iter()
}
