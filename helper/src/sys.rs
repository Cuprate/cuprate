//! System related

//---------------------------------------------------------------------------------------------------- Use
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

//---------------------------------------------------------------------------------------------------- Public API
#[inline]
/// Returns the current system time as a UNIX timestamp.
///
/// ```rust
/// # use helper::sys::*;
/// assert!(current_time() > 0);
/// ```
///
/// # Panics
/// This function panics if the call to get the system time fails.
pub fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[inline]
/// Returns the current system time as a UNIX timestamp in a [`Result`].
///
/// This function returns [`Err`] if the call to get the system time fails.
///
/// ```rust
/// # use helper::sys::*;
/// assert!(current_time_try().unwrap() > 0);
/// ```
pub fn current_time_try() -> Result<u64, SystemTimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .and_then(|d| Ok(d.as_secs()))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
