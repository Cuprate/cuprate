//! System thread related
//!
//! Requires `std`.

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
#[allow(non_snake_case)]
/// Get the total amount of system threads.
///
/// ```rust
/// # use helper::thread::*;
/// assert!(threads().get() >= 1);
/// ```
pub fn threads() -> NonZeroUsize {
	std::thread::available_parallelism().unwrap_or(NON_ZERO_USIZE_1)
}

// Implement a function for the various
// `x` thread-percent functions below.
macro_rules! impl_thread_percent {
    ($(
    	$(#[$doc:meta])*
    	$fn_name:ident => // Name of the function
		$percent:literal  // The target percent of threads
	),* $(,)?) => {
		$(
			$(#[$doc])*
			pub fn $fn_name() -> NonZeroUsize {
		        // SAFETY:
		        // unwrap here is okay because:
		        // - THREADS().get() is always non-zero
		        // - max() guards against 0
		        NonZeroUsize::new(max(1, (threads().get() as f64 * $percent).floor() as usize)).unwrap()
		    }
		)*
    }
}
impl_thread_percent! {
	/// Get 90% (rounded down) of available amount of system threads.
	threads_90 => 0.90,
	/// Get 75% (rounded down) of available amount of system threads.
	threads_75 => 0.75,
	/// Get 50% (rounded down) of available amount of system threads.
	threads_50 => 0.50,
	/// Get 25% (rounded down) of available amount of system threads.
	threads_25 => 0.25,
	/// Get 10% (rounded down) of available amount of system threads.
	threads_10 => 0.10,
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {}
