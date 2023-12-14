//! System thread related

//---------------------------------------------------------------------------------------------------- Use
use std::cmp::max;
use std::num::NonZeroUsize;
use std::sync::OnceLock;

//---------------------------------------------------------------------------------------------------- Constants
// FIXME: switch to `.unwrap()` when const stablized
const NON_ZERO_USIZE_1: NonZeroUsize = match NonZeroUsize::new(1) {
    Some(t) => t,
    _ => panic!(),
};

//----------------------------------------------------------------------------------------------------
// FIXME: switch to `LazyLock` when stablized
//
// INVARIANT:
// The other functions depend on this being set by the below
// `THREADS()` function, by the actual `available_parallelism()`
// function. This should be private and never set by anyone outside.
static THREADS_CELL: OnceLock<NonZeroUsize> = OnceLock::new();
#[allow(non_snake_case)]
/// Get the available amount of system threads.
///
/// This is lazily evaluated and returns 1 on errors.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(THREADS().get() >= 1);
/// ```
pub fn THREADS() -> NonZeroUsize {
    *THREADS_CELL.get_or_init(|| std::thread::available_parallelism().unwrap_or(NON_ZERO_USIZE_1))
}

// Implement the body for the various
// `X` thread-percent functions below.
macro_rules! impl_thread_percent {
    (
		$static:ident,   // The static holding the result
		$percent:literal // The target percent of threads
	) => {
        *$static.get_or_init(|| {
            let t = THREADS().get();
            // SAFETY:
            // unwrap here is okay because:
            // - THREADS().get() is always non-zero
            // - max() guards against 0
            NonZeroUsize::new(max(1, (t as f64 * $percent).floor() as usize)).unwrap()
        })
    };
}

// TODO: switch to `LazyLock` when stablized
static THREADS_10_CELL: OnceLock<NonZeroUsize> = OnceLock::new();
#[allow(non_snake_case)]
/// Get `10%` (rounded down) of available amount of system threads.
///
/// This is lazily evaluated and returns 1 on errors.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(THREADS_10().get() >= 1);
/// ```
pub fn THREADS_10() -> NonZeroUsize {
    impl_thread_percent!(THREADS_10_CELL, 0.10)
}

// TODO: switch to `LazyLock` when stablized
static THREADS_25_CELL: OnceLock<NonZeroUsize> = OnceLock::new();
#[allow(non_snake_case)]
/// Get `25%` (rounded down) of available amount of system threads.
///
/// This is lazily evaluated and returns 1 on errors.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(THREADS_25().get() >= 1);
/// ```
pub fn THREADS_25() -> NonZeroUsize {
    impl_thread_percent!(THREADS_25_CELL, 0.25)
}

// TODO: switch to `LazyLock` when stablized
static THREADS_50_CELL: OnceLock<NonZeroUsize> = OnceLock::new();
#[allow(non_snake_case)]
/// Get `50%` (rounded down) the available amount of system threads.
///
/// This is lazily evaluated and returns 1 on errors.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(THREADS_50().get() >= 1);
/// ```
pub fn THREADS_50() -> NonZeroUsize {
    impl_thread_percent!(THREADS_50_CELL, 0.50)
}

// TODO: switch to `LazyLock` when stablized
static THREADS_75_CELL: OnceLock<NonZeroUsize> = OnceLock::new();
#[allow(non_snake_case)]
/// Get `75%` (rounded down) of available amount of system threads.
///
/// This is lazily evaluated and returns 1 on errors.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(THREADS_75().get() >= 1);
/// ```
pub fn THREADS_75() -> NonZeroUsize {
    impl_thread_percent!(THREADS_75_CELL, 0.75)
}

// TODO: switch to `LazyLock` when stablized
static THREADS_90_CELL: OnceLock<NonZeroUsize> = OnceLock::new();
#[allow(non_snake_case)]
/// Get `90%` (rounded down) of available amount of system threads.
///
/// This is lazily evaluated and returns 1 on errors.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(THREADS_90().get() >= 1);
/// ```
pub fn THREADS_90() -> NonZeroUsize {
    impl_thread_percent!(THREADS_90_CELL, 0.90)
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Assert that the `NonZeroUsize` constants are correct.
    fn non_zero_usize() {
        assert_eq!(NON_ZERO_USIZE_1.get(), 1);
    }

    #[test]
    // Assert thread division functions return
    // the expected divided thread count.
    fn thread_division_1() {
        THREADS_CELL.set(NonZeroUsize::new(100).unwrap()).unwrap();
        assert_eq!(THREADS().get(), 100);
        assert_eq!(THREADS_90().get(), 90);
        assert_eq!(THREADS_75().get(), 75);
        assert_eq!(THREADS_50().get(), 50);
        assert_eq!(THREADS_25().get(), 25);
        assert_eq!(THREADS_10().get(), 10);
    }
}
