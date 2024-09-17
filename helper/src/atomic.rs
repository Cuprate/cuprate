//! Atomic related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use crossbeam::atomic::AtomicCell;

//---------------------------------------------------------------------------------------------------- Atomic Float
/// Compile-time assertion that our floats are
/// lock-free for the target we're building for.
const _: () = {
    assert!(
        AtomicCell::<f32>::is_lock_free(),
        "32-bit atomics are not supported on this build target."
    );

    assert!(
        AtomicCell::<f64>::is_lock_free(),
        "64-bit atomics are not supported on this build target."
    );
};

// SOMEDAY: use a custom float that implements `Eq`
// so that `compare_exchange()`, `fetch_*()` work.

/// An atomic [`f32`].
///
/// This is an alias for
/// [`crossbeam::atomic::AtomicCell<f32>`](https://docs.rs/crossbeam/latest/crossbeam/atomic/struct.AtomicCell.html).
///
/// Note that there are no [Ordering] parameters,
/// atomic loads use [Acquire],
/// and atomic stores use [Release].
///
/// [Ordering]: std::sync::atomic::Ordering
/// [Acquire]: std::sync::atomic::Ordering::Acquire
/// [Release]: std::sync::atomic::Ordering::Release
pub type AtomicF32 = AtomicCell<f32>;

/// An atomic [`f64`].
///
/// This is an alias for
/// [`crossbeam::atomic::AtomicCell<f64>`](https://docs.rs/crossbeam/latest/crossbeam/atomic/struct.AtomicCell.html).
///
/// Note that there are no [Ordering] parameters,
/// atomic loads use [Acquire],
/// and atomic stores use [Release].
///
/// [Ordering]: std::sync::atomic::Ordering
/// [Acquire]: std::sync::atomic::Ordering::Acquire
/// [Release]: std::sync::atomic::Ordering::Release
pub type AtomicF64 = AtomicCell<f64>;

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    // Tests `AtomicF32`.
    fn f32() {
        let float = AtomicF32::new(5.0);

        // Loads/Stores
        assert_eq!(float.swap(1.0), 5.0);
        assert_eq!(float.load(), 1.0);
        float.store(2.0);
        assert_eq!(float.load(), 2.0);
    }

    #[test]
    // Tests `AtomicF64`.
    fn f64() {
        let float = AtomicF64::new(5.0);

        // Loads/Stores
        assert_eq!(float.swap(1.0), 5.0);
        assert_eq!(float.load(), 1.0);
        float.store(2.0);
        assert_eq!(float.load(), 2.0);
    }
}
