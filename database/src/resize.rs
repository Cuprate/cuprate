//! Database memory map resizing algorithms.
//!
//! TODO.

//---------------------------------------------------------------------------------------------------- Import
use std::num::NonZeroUsize;

#[allow(unused_imports)] // docs
use crate::{env::Env, ConcreteEnv};

//---------------------------------------------------------------------------------------------------- ResizeAlgorithm
/// The function/algorithm used by the
/// database when resizing the memory map.
///
/// This is only used by [`ConcreteEnv`] if [`Env::MANUAL_RESIZE`] is `true`.
///
/// # TODO
/// We could test around with different algorithms.
/// Calling [`heed::Env::resize`] is surprisingly fast,
/// around `0.0000082s` on my machine. We could probably
/// get away with smaller and more frequent resizes.
/// **With the caveat being we are taking a `WriteGuard` to a `RwLock`.**
#[derive(Copy, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum ResizeAlgorithm {
    /// TODO
    Monero,
    /// TODO
    FixedBytes(NonZeroUsize),
    /// TODO
    Percent(f32),
}

impl ResizeAlgorithm {
    /// TODO
    pub const fn new() -> Self {
        Self::Monero
    }

    /// TODO
    pub fn resize(&self, current_size_bytes: usize) -> NonZeroUsize {
        match self {
            Self::Monero => monero(current_size_bytes),
            Self::FixedBytes(u) => todo!(),
            Self::Percent(f) => todo!(),
        }
    }
}

impl Default for ResizeAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

//---------------------------------------------------------------------------------------------------- Free functions
// `page_size` itself caches the result, so we don't need to,
// this function is cheap after 1st call: <https://docs.rs/page_size>.
pub use page_size::get as page_size;

/// Memory map resize closely matching `monerod`.
///
/// # Method
/// This function mostly matches `monerod`'s current resize implementation[^1],
/// and will increase `current_size_bytes` by `1 << 30`[^2] exactly then
/// rounded to the nearest multiple of the OS page size.
///
/// [^1]: <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L549>
///
/// [^2]: `1_073_745_920`
///
/// ```rust
/// # use cuprate_database::resize::*;
/// // The value this function will increment by
/// // (assuming page multiple of 4096).
/// const N: usize = 1_073_741_824;
///
/// // 0 returns the minimum value.
/// assert_eq!(monero(0).get(), N);
/// // Rounds up to nearest OS page size.
/// assert_eq!(monero(1).get(), N + page_size());
/// ```
#[allow(clippy::missing_panics_doc)] // Can't panic.
pub fn monero(current_size_bytes: usize) -> NonZeroUsize {
    /// The exact expression used by `monerod`
    /// when calculating how many bytes to add.
    ///
    /// The nominal value is `1_073_741_824`.
    /// Not actually 1 GB but close enough I guess.
    ///
    /// <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L553>
    const ADD_SIZE: usize = 1_usize << 30;

    // SAFETY: If this overflows, we should definitely panic.
    // `u64::MAX` bytes is... ~18,446,744 terabytes.
    let new_size_bytes = current_size_bytes + ADD_SIZE;

    // INVARIANT: Round up to the nearest OS page size multiple.
    // LMDB/heed will error if this is not the case:
    // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
    let page_size = page_size();

    // Round up the new size to the
    // nearest multiple of the OS page size.
    let remainder = new_size_bytes % page_size;

    // SAFETY: minimum is always at least `ADD_SIZE`.
    NonZeroUsize::new(if remainder == 0 {
        new_size_bytes
    } else {
        (new_size_bytes + page_size) - remainder
    })
    .unwrap()
}

/// TODO
pub fn fixed_bytes(current_size_bytes: usize, bytes: usize) -> NonZeroUsize {
    todo!()
}

/// TODO
pub fn percent(current_size_bytes: usize, percent: f32) -> NonZeroUsize {
    todo!()
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
