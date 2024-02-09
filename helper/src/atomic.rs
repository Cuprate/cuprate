//! Atomic related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

//---------------------------------------------------------------------------------------------------- Atomic Float
// An AtomicF(32|64) implementation.
//
// This internally uses [AtomicU(32|64)], where the
// u(32|64) is the bit pattern of the internal float.
//
// This uses [.to_bits()] and [from_bits()] to
// convert between actual floats, and the bit
// representations for storage.
//
// Using `UnsafeCell<float>` is also viable,
// and would allow for a `const fn new(f: float) -> Self`
// except that becomes problematic with NaN's and infinites:
// - <https://github.com/rust-lang/rust/issues/73328>
// - <https://github.com/rust-lang/rfcs/pull/3514>
//
// This is most likely safe(?) but... instead of risking UB,
// this just uses the Atomic unsigned integer as the inner
// type instead of transmuting from `UnsafeCell`.
//
// This creates the types:
// - `AtomicF32`
// - `AtomicF64`
//
// Originally taken from:
// <https://github.com/hinto-janai/sansan/blob/1f6680b2d08ff5fbf4f090178ea5233d4cf9056f/src/atomic.rs>
macro_rules! impl_atomic_f {
    (
		$atomic_float:ident,       // Name of the new float type
		$atomic_float_lit:literal, // Literal name of new float type
		$float:ident,              // The target float (f32/f64)
		$unsigned:ident,           // The underlying unsigned type
		$atomic_unsigned:ident,    // The underlying unsigned atomic type
		$bits_0:literal,           // Bit pattern for 0.0
		$bits_025:literal,         // Bit pattern for 0.25
		$bits_050:literal,         // Bit pattern for 0.50
		$bits_075:literal,         // Bit pattern for 0.75
		$bits_1:literal,           // Bit pattern for 1.0
	) => {
        /// An atomic float.
        ///
        /// ## Portability
        /// [Quoting the std library: ](<https://doc.rust-lang.org/1.70.0/std/primitive.f32.html#method.to_bits)>
        /// "See from_bits for some discussion of the portability of this operation (there are almost no issues)."
        ///
        /// ## Compile-time failure
        /// This internal functions `std` uses will panic _at compile time_
        /// if the bit transmutation operations it uses are not available
        /// on the build target, aka, if it compiles we're probably safe.
        #[repr(transparent)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
        pub struct $atomic_float($atomic_unsigned);

        impl $atomic_float {
            /// Representation of `0.0` as bits, can be inputted into [`Self::from_bits`].
            pub const BITS_0: $unsigned = $bits_0;
            /// Representation of `0.25` as bits, can be inputted into [`Self::from_bits`].
            pub const BITS_0_25: $unsigned = $bits_025;
            /// Representation of `0.50` as bits, can be inputted into [`Self::from_bits`].
            pub const BITS_0_50: $unsigned = $bits_050;
            /// Representation of `0.75` as bits, can be inputted into [`Self::from_bits`].
            pub const BITS_0_75: $unsigned = $bits_075;
            /// Representation of `1.0` as bits, can be inputted into [`Self::from_bits`].
            pub const BITS_0_100: $unsigned = $bits_1;

            #[allow(clippy::declare_interior_mutable_const)]
            // FIXME:
            // Seems like `std` internals has some unstable cfg options that
            // allow interior mutable consts to be defined without clippy complaining:
            // <https://doc.rust-lang.org/1.70.0/src/core/sync/atomic.rs.html#3013>.
            //
            /// `0.0`, returned by [`Self::default`].
            pub const DEFAULT: Self = Self($atomic_unsigned::new($bits_0));

            #[inline]
            /// Create a new atomic float.
            ///
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.new>
            pub fn new(f: $float) -> Self {
                // FIXME: Update to const when available.
                // <https://doc.rust-lang.org/1.70.0/src/core/num/f32.rs.html#998>
                //
                // `transmute()` here would be safe (`to_bits()` is doing this)
                // although checking for NaN's and infinites are non-`const`...
                // so we can't can't `transmute()` even though it would allow
                // this function to be `const`.
                Self($atomic_unsigned::new(f.to_bits()))
            }

            #[inline]
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.into_inner>
            pub fn into_inner(self) -> $float {
                $float::from_bits(self.0.into_inner())
            }

            #[inline]
            /// Create a new atomic float, from the unsigned bit representation.
            pub const fn from_bits(bits: $unsigned) -> Self {
                Self($atomic_unsigned::new(bits))
            }

            #[inline]
            /// Store a float inside the atomic.
            ///
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.store>
            pub fn store(&self, f: $float, ordering: Ordering) {
                self.0.store(f.to_bits(), ordering);
            }

            #[inline]
            /// Store a bit representation of a float inside the atomic.
            pub fn store_bits(&self, bits: $unsigned, ordering: Ordering) {
                self.0.store(bits, ordering);
            }

            #[inline]
            /// Load the internal float from the atomic.
            ///
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.load>
            pub fn load(&self, ordering: Ordering) -> $float {
                // FIXME: Update to const when available.
                // <https://doc.rust-lang.org/1.70.0/src/core/num/f32.rs.html#1088>
                $float::from_bits(self.0.load(ordering))
            }

            #[inline]
            /// Load the internal bit representation of the float from the atomic.
            pub fn load_bits(&self, ordering: Ordering) -> $unsigned {
                self.0.load(ordering)
            }

            #[inline]
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.swap>
            pub fn swap(&self, val: $float, ordering: Ordering) -> $float {
                $float::from_bits(self.0.swap($float::to_bits(val), ordering))
            }

            #[inline]
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.compare_exchange>
            pub fn compare_exchange(
                &self,
                current: $float,
                new: $float,
                success: Ordering,
                failure: Ordering,
            ) -> Result<$float, $float> {
                match self
                    .0
                    .compare_exchange(current.to_bits(), new.to_bits(), success, failure)
                {
                    Ok(b) => Ok($float::from_bits(b)),
                    Err(b) => Err($float::from_bits(b)),
                }
            }

            #[inline]
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.compare_exchange_weak>
            pub fn compare_exchange_weak(
                &self,
                current: $float,
                new: $float,
                success: Ordering,
                failure: Ordering,
            ) -> Result<$float, $float> {
                match self.0.compare_exchange_weak(
                    current.to_bits(),
                    new.to_bits(),
                    success,
                    failure,
                ) {
                    Ok(b) => Ok($float::from_bits(b)),
                    Err(b) => Err($float::from_bits(b)),
                }
            }

            //------------------------------------------------------------------ fetch_*()
            // These are tricky to implement because we must
            // operate on the _numerical_ value and not the
            // bit representations.
            //
            // This means using some type of CAS,
            // which comes with the regular tradeoffs...

            // The (private) function using CAS to implement `fetch_*()` operations.
            //
            // This is function body used in all the below `fetch_*()` functions.
            fn fetch_update_unwrap<F>(&self, ordering: Ordering, mut update: F) -> $float
            where
                F: FnMut($float) -> $float,
            {
                // Since it's a CAS, we need a second ordering for failures,
                // this will take the user input and return an appropriate order.
                let second_order = match ordering {
                    Ordering::Release | Ordering::Relaxed => Ordering::Relaxed,
                    Ordering::Acquire | Ordering::AcqRel => Ordering::Acquire,
                    Ordering::SeqCst => Ordering::SeqCst,
                    // Ordering is #[non_exhaustive], so we must do this.
                    ordering => ordering,
                };

                // SAFETY:
                // unwrap is safe since `fetch_update()` only panics
                // if the closure we pass it returns `None`.
                // As seen below, we're passing a `Some`.
                //
                // <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.fetch_update>
                self.fetch_update(ordering, second_order, |f| Some(update(f)))
                    .unwrap()
            }

            #[inline]
            /// This function is implemented with [`Self::fetch_update`], and is not 100% equivalent to
            /// <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.fetch_add>.
            ///
            /// In particular, this method will not circumvent the [ABA Problem](https://en.wikipedia.org/wiki/ABA_problem).
            ///
            /// Other than this not actually being atomic, all other behaviors are the same.
            pub fn fetch_add(&self, val: $float, order: Ordering) -> $float {
                self.fetch_update_unwrap(order, |f| f + val)
            }

            #[inline]
            /// This function is implemented with [`Self::fetch_update`], and is not 100% equivalent to
            /// <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.fetch_sub>.
            ///
            /// In particular, this method will not circumvent the [ABA Problem](https://en.wikipedia.org/wiki/ABA_problem).
            ///
            /// Other than this not actually being atomic, all other behaviors are the same.
            pub fn fetch_sub(&self, val: $float, order: Ordering) -> $float {
                self.fetch_update_unwrap(order, |f| f - val)
            }

            #[inline]
            /// This function is implemented with [`Self::fetch_update`], and is not 100% equivalent to
            /// <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.fetch_max>.
            ///
            /// In particular, this method will not circumvent the [ABA Problem](https://en.wikipedia.org/wiki/ABA_problem).
            ///
            /// Other than this not actually being atomic, all other behaviors are the same.
            pub fn fetch_max(&self, val: $float, order: Ordering) -> $float {
                self.fetch_update_unwrap(order, |f| f.max(val))
            }

            #[inline]
            /// This function is implemented with [`Self::fetch_update`], and is not 100% equivalent to
            /// <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.fetch_min>.
            ///
            /// In particular, this method will not circumvent the [ABA Problem](https://en.wikipedia.org/wiki/ABA_problem).
            ///
            /// Other than this not actually being atomic, all other behaviors are the same.
            pub fn fetch_min(&self, val: $float, order: Ordering) -> $float {
                self.fetch_update_unwrap(order, |f| f.min(val))
            }

            #[inline]
            /// Equivalent to <https://doc.rust-lang.org/1.70.0/std/sync/atomic/struct.AtomicUsize.html#method.fetch_update>
            pub fn fetch_update<F>(
                &self,
                set_order: Ordering,
                fetch_order: Ordering,
                mut f: F,
            ) -> Result<$float, $float>
            where
                F: FnMut($float) -> Option<$float>,
            {
                // Very unreadable closure...
                //
                // Basically this is converting:
                //   `f(f32) -> Option<f32>` into `f(u32) -> Option<u32>`
                // so the internal atomic `fetch_update` can work.
                let f = |bits: $unsigned| f($float::from_bits(bits)).map(|f| $float::to_bits(f));

                match self.0.fetch_update(set_order, fetch_order, f) {
                    Ok(b) => Ok($float::from_bits(b)),
                    Err(b) => Err($float::from_bits(b)),
                }
            }

            #[inline]
            /// Set the internal float from the atomic, using [`Ordering::Release`].
            pub fn set(&self, f: $float) {
                self.store(f, Ordering::Release);
            }

            #[inline]
            /// Get the internal float from the atomic, using [`Ordering::Acquire`].
            pub fn get(&self) -> $float {
                self.load(Ordering::Acquire)
            }
        }

        impl From<$float> for $atomic_float {
            /// Calls [`Self::new`]
            fn from(float: $float) -> Self {
                Self::new(float)
            }
        }

        impl Default for $atomic_float {
            /// Returns `0.0`.
            fn default() -> Self {
                Self::DEFAULT
            }
        }

        impl std::fmt::Debug for $atomic_float {
            /// This prints the internal float value, using [`Ordering::Acquire`].
            ///
            /// # Panics
            /// This panics on NaN or subnormal float inputs.
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple($atomic_float_lit)
                    .field(&self.0.load(Ordering::Acquire))
                    .finish()
            }
        }
    };
}

impl_atomic_f! {
    AtomicF64,
    "AtomicF64",
    f64,
    u64,
    AtomicU64,
    0,
    4598175219545276416,
    4602678819172646912,
    4604930618986332160,
    4607182418800017408,
}

impl_atomic_f! {
    AtomicF32,
    "AtomicF32",
    f32,
    u32,
    AtomicU32,
    0,
    1048576000,
    1056964608,
    1061158912,
    1065353216,
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    use super::*;

    // These tests come in pairs, `f32|f64`.
    //
    // If changing one, update the other as well.
    //
    // `macro_rules!()` + `paste!()` could do this automatically,
    // but that might be more trouble than it's worth...

    #[test]
    // Tests the varying fetch, swap, and compare functions.
    fn f32_functions() {
        let float = AtomicF32::new(5.0);
        let ordering = Ordering::SeqCst;

        // Loads/Stores
        assert_eq!(float.swap(1.0, ordering), 5.0);
        assert_eq!(float.load(ordering), 1.0);
        float.store(2.0, ordering);
        assert_eq!(float.load(ordering), 2.0);

        // CAS
        assert_eq!(
            float.compare_exchange(2.0, 5.0, ordering, ordering),
            Ok(2.0)
        );
        assert_eq!(
            float.fetch_update(ordering, ordering, |f| Some(f * 3.0)),
            Ok(5.0)
        );
        assert_eq!(float.get(), 15.0);
        loop {
            if let Ok(float) = float.compare_exchange_weak(15.0, 2.0, ordering, ordering) {
                assert_eq!(float, 15.0);
                break;
            }
        }

        // `fetch_*()` functions
        assert_eq!(float.fetch_add(1.0, ordering), 2.0);
        assert_eq!(float.fetch_sub(1.0, ordering), 3.0);
        assert_eq!(float.fetch_max(5.0, ordering), 2.0);
        assert_eq!(float.fetch_min(0.0, ordering), 5.0);
    }

    #[test]
    fn f64_functions() {
        let float = AtomicF64::new(5.0);
        let ordering = Ordering::SeqCst;

        assert_eq!(float.swap(1.0, ordering), 5.0);
        assert_eq!(float.load(ordering), 1.0);
        float.store(2.0, ordering);
        assert_eq!(float.load(ordering), 2.0);

        assert_eq!(
            float.compare_exchange(2.0, 5.0, ordering, ordering),
            Ok(2.0)
        );
        assert_eq!(
            float.fetch_update(ordering, ordering, |f| Some(f * 3.0)),
            Ok(5.0)
        );
        assert_eq!(float.get(), 15.0);

        loop {
            if let Ok(float) = float.compare_exchange_weak(15.0, 2.0, ordering, ordering) {
                assert_eq!(float, 15.0);
                break;
            }
        }

        assert_eq!(float.fetch_add(1.0, ordering), 2.0);
        assert_eq!(float.fetch_sub(1.0, ordering), 3.0);
        assert_eq!(float.fetch_max(5.0, ordering), 2.0);
        assert_eq!(float.fetch_min(0.0, ordering), 5.0);
    }

    #[test]
    fn f32_bits() {
        assert_eq!(AtomicF32::default().get(), 0.00);
        assert_eq!(AtomicF32::from_bits(AtomicF32::BITS_0).get(), 0.00);
        assert_eq!(AtomicF32::from_bits(AtomicF32::BITS_0_25).get(), 0.25);
        assert_eq!(AtomicF32::from_bits(AtomicF32::BITS_0_50).get(), 0.50);
        assert_eq!(AtomicF32::from_bits(AtomicF32::BITS_0_75).get(), 0.75);
        assert_eq!(AtomicF32::from_bits(AtomicF32::BITS_0_100).get(), 1.00);
    }

    #[test]
    fn f64_bits() {
        assert_eq!(AtomicF64::default().get(), 0.00);
        assert_eq!(AtomicF64::from_bits(AtomicF64::BITS_0).get(), 0.00);
        assert_eq!(AtomicF64::from_bits(AtomicF64::BITS_0_25).get(), 0.25);
        assert_eq!(AtomicF64::from_bits(AtomicF64::BITS_0_50).get(), 0.50);
        assert_eq!(AtomicF64::from_bits(AtomicF64::BITS_0_75).get(), 0.75);
        assert_eq!(AtomicF64::from_bits(AtomicF64::BITS_0_100).get(), 1.00);
    }

    #[test]
    fn f32_0_to_100() {
        let mut i = 0.0;
        let f = AtomicF32::new(0.0);
        while i < 100.0 {
            f.set(i);
            assert_eq!(f.get(), i);
            i += 0.1;
        }
    }

    #[test]
    fn f64_0_to_100() {
        let mut i = 0.0;
        let f = AtomicF64::new(0.0);
        while i < 100.0 {
            f.set(i);
            assert_eq!(f.get(), i);
            i += 0.1;
        }
    }

    #[test]
    fn f32_irregular() {
        assert!(AtomicF32::new(f32::NAN).get().is_nan());
        assert_eq!(AtomicF32::new(f32::INFINITY).get(), f32::INFINITY);
        assert_eq!(AtomicF32::new(f32::NEG_INFINITY).get(), f32::NEG_INFINITY);
    }

    #[test]
    fn f64_irregular() {
        assert!(AtomicF64::new(f64::NAN).get().is_nan());
        assert_eq!(AtomicF64::new(f64::INFINITY).get(), f64::INFINITY);
        assert_eq!(AtomicF64::new(f64::NEG_INFINITY).get(), f64::NEG_INFINITY);
    }
}
