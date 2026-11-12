//! System related
//!
//! Requires `std`.

//---------------------------------------------------------------------------------------------------- Use
use std::time::{SystemTime, UNIX_EPOCH};

//---------------------------------------------------------------------------------------------------- Public API
#[inline]
/// Returns the current system time as a UNIX timestamp.
///
/// ```rust
/// # use cuprate_helper::time::*;
/// assert!(current_unix_timestamp() > 0);
/// ```
///
/// # Panics
/// This function panics if the call to get the system time fails.
pub fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
