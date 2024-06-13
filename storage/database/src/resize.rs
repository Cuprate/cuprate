//! Database memory map resizing algorithms.
//!
//! This modules contains [`ResizeAlgorithm`] which determines how the
//! [`ConcreteEnv`](crate::ConcreteEnv) resizes its memory map when needing more space.
//! This value is in [`Config`](crate::config::Config) and can be selected at runtime.
//!
//! Although, it is only used by `ConcreteEnv` if [`Env::MANUAL_RESIZE`](crate::env::Env::MANUAL_RESIZE) is `true`.
//!
//! The algorithms are available as free functions in this module as well.
//!
//! # Page size
//! All free functions in this module will
//! return a multiple of the OS page size ([`page_size()`]),
//! [LMDB will error](http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5)
//! if this is not the case.
//!
//! # Invariants
//! All returned [`NonZeroUsize`] values of the free functions in this module
//! (including [`ResizeAlgorithm::resize`]) uphold the following invariants:
//! 1. It will always be `>=` the input `current_size_bytes`
//! 2. It will always be a multiple of [`page_size()`]

//---------------------------------------------------------------------------------------------------- Import
use std::{num::NonZeroUsize, sync::OnceLock};

//---------------------------------------------------------------------------------------------------- ResizeAlgorithm
/// The function/algorithm used by the
/// database when resizing the memory map.
///
// # SOMEDAY
// We could test around with different algorithms.
// Calling `heed::Env::resize` is surprisingly fast,
// around `0.0000082s` on my machine. We could probably
// get away with smaller and more frequent resizes.
// **With the caveat being we are taking a `WriteGuard` to a `RwLock`.**
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResizeAlgorithm {
    /// Uses [`monero`].
    Monero,

    /// Uses [`fixed_bytes`].
    FixedBytes(NonZeroUsize),

    /// Uses [`percent`].
    Percent(f32),
}

impl ResizeAlgorithm {
    /// Returns [`Self::Monero`].
    ///
    /// ```rust
    /// # use database::resize::*;
    /// assert!(matches!(ResizeAlgorithm::new(), ResizeAlgorithm::Monero));
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self::Monero
    }

    /// Maps the `self` variant to the free functions in [`crate::resize`].
    ///
    /// This function returns the _new_ memory map size in bytes.
    #[inline]
    pub fn resize(&self, current_size_bytes: usize) -> NonZeroUsize {
        match self {
            Self::Monero => monero(current_size_bytes),
            Self::FixedBytes(add_bytes) => fixed_bytes(current_size_bytes, add_bytes.get()),
            Self::Percent(f) => percent(current_size_bytes, *f),
        }
    }
}

impl Default for ResizeAlgorithm {
    /// Calls [`Self::new`].
    ///
    /// ```rust
    /// # use database::resize::*;
    /// assert_eq!(ResizeAlgorithm::new(), ResizeAlgorithm::default());
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

//---------------------------------------------------------------------------------------------------- Free functions
/// This function retrieves the system’s memory page size.
///
/// It is just [`page_size::get`](https://docs.rs/page_size) internally.
///
/// This caches the result, so this function is cheap after the 1st call.
///
/// # Panics
/// This function will panic if the OS returns of page size of `0` (impossible?).
#[inline]
pub fn page_size() -> NonZeroUsize {
    /// Cached result of [`page_size()`].
    static PAGE_SIZE: OnceLock<NonZeroUsize> = OnceLock::new();
    *PAGE_SIZE
        .get_or_init(|| NonZeroUsize::new(page_size::get()).expect("page_size::get() returned 0"))
}

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
/// # use database::resize::*;
/// // The value this function will increment by
/// // (assuming page multiple of 4096).
/// const N: usize = 1_073_741_824;
///
/// // 0 returns the minimum value.
/// assert_eq!(monero(0).get(), N);
///
/// // Rounds up to nearest OS page size.
/// assert_eq!(monero(1).get(), N + page_size().get());
/// ```
///
/// # Panics
/// This function will panic if adding onto `current_size_bytes` overflows [`usize::MAX`].
///
/// ```rust,should_panic
/// # use database::resize::*;
/// // Ridiculous large numbers panic.
/// monero(usize::MAX);
/// ```
pub fn monero(current_size_bytes: usize) -> NonZeroUsize {
    /// The exact expression used by `monerod`
    /// when calculating how many bytes to add.
    ///
    /// The nominal value is `1_073_741_824`.
    /// Not actually 1 GB but close enough I guess.
    ///
    /// <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L553>
    const ADD_SIZE: usize = 1_usize << 30;

    let page_size = page_size().get();
    let new_size_bytes = current_size_bytes + ADD_SIZE;

    // Round up the new size to the
    // nearest multiple of the OS page size.
    let remainder = new_size_bytes % page_size;

    // INVARIANT: minimum is always at least `ADD_SIZE`.
    NonZeroUsize::new(if remainder == 0 {
        new_size_bytes
    } else {
        (new_size_bytes + page_size) - remainder
    })
    .unwrap()
}

/// Memory map resize by a fixed amount of bytes.
///
/// # Method
/// This function will `current_size_bytes + add_bytes`
/// and then round up to nearest OS page size.
///
/// ```rust
/// # use database::resize::*;
/// let page_size: usize = page_size().get();
///
/// // Anything below the page size will round up to the page size.
/// for i in 0..=page_size {
///     assert_eq!(fixed_bytes(0, i).get(), page_size);
/// }
///
/// // (page_size + 1) will round up to (page_size * 2).
/// assert_eq!(fixed_bytes(page_size, 1).get(), page_size * 2);
///
/// // (page_size + page_size) doesn't require any rounding.
/// assert_eq!(fixed_bytes(page_size, page_size).get(), page_size * 2);
/// ```
///
/// # Panics
/// This function will panic if adding onto `current_size_bytes` overflows [`usize::MAX`].
///
/// ```rust,should_panic
/// # use database::resize::*;
/// // Ridiculous large numbers panic.
/// fixed_bytes(1, usize::MAX);
/// ```
pub fn fixed_bytes(current_size_bytes: usize, add_bytes: usize) -> NonZeroUsize {
    let page_size = page_size();
    let new_size_bytes = current_size_bytes + add_bytes;

    // Guard against < page_size.
    if new_size_bytes <= page_size.get() {
        return page_size;
    }

    // Round up the new size to the
    // nearest multiple of the OS page size.
    let remainder = new_size_bytes % page_size;

    // INVARIANT: we guarded against < page_size above.
    NonZeroUsize::new(if remainder == 0 {
        new_size_bytes
    } else {
        (new_size_bytes + page_size.get()) - remainder
    })
    .unwrap()
}

/// Memory map resize by a percentage.
///
/// # Method
/// This function will multiply `current_size_bytes` by `percent`.
///
/// Any input `<= 1.0` or non-normal float ([`f32::NAN`], [`f32::INFINITY`])
/// will make the returning `NonZeroUsize` the same as `current_size_bytes`
/// (rounded up to the OS page size).
///
/// ```rust
/// # use database::resize::*;
/// let page_size: usize = page_size().get();
///
/// // Anything below the page size will round up to the page size.
/// for i in 0..=page_size {
///     assert_eq!(percent(i, 1.0).get(), page_size);
/// }
///
/// // Same for 2 page sizes.
/// for i in (page_size + 1)..=(page_size * 2) {
///     assert_eq!(percent(i, 1.0).get(), page_size * 2);
/// }
///
/// // Weird floats do nothing.
/// assert_eq!(percent(page_size, f32::NAN).get(), page_size);
/// assert_eq!(percent(page_size, f32::INFINITY).get(), page_size);
/// assert_eq!(percent(page_size, f32::NEG_INFINITY).get(), page_size);
/// assert_eq!(percent(page_size, -1.0).get(), page_size);
/// assert_eq!(percent(page_size, 0.999).get(), page_size);
/// ```
///
/// # Panics
/// This function will panic if `current_size_bytes * percent`
/// is closer to [`usize::MAX`] than the OS page size.
///
/// ```rust,should_panic
/// # use database::resize::*;
/// // Ridiculous large numbers panic.
/// percent(usize::MAX, 1.001);
/// ```
pub fn percent(current_size_bytes: usize, percent: f32) -> NonZeroUsize {
    // Guard against bad floats.
    use std::num::FpCategory;
    let percent = match percent.classify() {
        FpCategory::Normal => {
            if percent <= 1.0 {
                1.0
            } else {
                percent
            }
        }
        _ => 1.0,
    };

    let page_size = page_size();

    // INVARIANT: Allow `f32` <-> `usize` casting, we handle all cases.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let new_size_bytes = ((current_size_bytes as f32) * percent) as usize;

    // Panic if rounding up to the nearest page size would overflow.
    let new_size_bytes = if new_size_bytes > (usize::MAX - page_size.get()) {
        panic!("new_size_bytes is percent() near usize::MAX");
    } else {
        new_size_bytes
    };

    // Guard against < page_size.
    if new_size_bytes <= page_size.get() {
        return page_size;
    }

    // Round up the new size to the
    // nearest multiple of the OS page size.
    let remainder = new_size_bytes % page_size;

    // INVARIANT: we guarded against < page_size above.
    NonZeroUsize::new(if remainder == 0 {
        new_size_bytes
    } else {
        (new_size_bytes + page_size.get()) - remainder
    })
    .unwrap()
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
