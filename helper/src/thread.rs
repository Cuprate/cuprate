//! System thread related
//!
//! Requires `std`.

//---------------------------------------------------------------------------------------------------- Use
use std::{cmp::max, num::NonZeroUsize};

//---------------------------------------------------------------------------------------------------- Thread Count & Percent
/// Get the total amount of system threads.
///
/// ```rust
/// # use cuprate_helper::thread::*;
/// assert!(threads().get() >= 1);
/// ```
pub fn threads() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
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
                // unwrap here is okay because:
                // - THREADS().get() is always non-zero
                // - max() guards against 0
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss,
                    reason = "we need to round integers"
                )]
		        NonZeroUsize::new(max(1, (threads().get() as f64 * $percent).ceil() as usize)).unwrap()
		    }
		)*
    }
}
impl_thread_percent! {
    /// Get 90% (rounded up) of available amount of system threads.
    threads_90 => 0.90,
    /// Get 75% (rounded up) of available amount of system threads.
    threads_75 => 0.75,
    /// Get 50% (rounded up) of available amount of system threads.
    threads_50 => 0.50,
    /// Get 25% (rounded up) of available amount of system threads.
    threads_25 => 0.25,
    /// Get 10% (rounded up) of available amount of system threads.
    threads_10 => 0.10,
}

//---------------------------------------------------------------------------------------------------- Thread Priority
/// Low Priority Thread
///
/// Sets the calling thread’s priority to the lowest platform-specific value possible.
///
/// Originally from <https://docs.rs/lpt>.
///
/// # Windows
/// Uses `SetThreadPriority()` with `THREAD_PRIORITY_IDLE` (-15).
///
/// # Unix
/// Uses `libc::nice()` with the max nice level.
///
/// On macOS and *BSD: +20
/// On Linux: +19
pub fn low_priority_thread() {
    #[cfg(target_os = "windows")]
    {
        use target_os_lib as windows;
        use windows::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_IDLE,
        };

        // SAFETY: calling C.
        // We are _lowering_ our priority, not increasing, so this function should never fail.
        unsafe {
            drop(SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_IDLE));
        }
    }

    #[cfg(target_family = "unix")]
    {
        use target_os_lib as libc;

        const NICE_MAX: libc::c_int = if cfg!(target_os = "linux") { 19 } else { 20 };

        // SAFETY: calling C.
        // We are _lowering_ our priority, not increasing, so this function should never fail.
        unsafe {
            libc::nice(NICE_MAX);
        }
    }
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {}
