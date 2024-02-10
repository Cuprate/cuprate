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

/// An atomic [`f32`].
pub type AtomicF32 = AtomicCell<f32>;

/// An atomic [`f64`].
pub type AtomicF64 = AtomicCell<f64>;

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
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
