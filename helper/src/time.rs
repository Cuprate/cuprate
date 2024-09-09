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

#[inline]
/// Get the clock time of a UNIX timestamp
///
/// The input must be a UNIX timestamp.
///
/// The returned `u64` will represent how many seconds has
/// passed on the day corresponding to that timestamp.
///
/// The output is guaranteed to be in the range of `0..=86399`.
///
/// ```rust
/// # use cuprate_helper::time::*;
/// // October 20th 2023 - 10:18:30 PM
/// const TIME: u64 = 1697840310;
///
/// let seconds = unix_clock(TIME);
/// assert_eq!(seconds, 80310);
///
/// let (h, m, s) = secs_to_clock(seconds);
/// // 10:18:30 PM.
/// assert_eq!((h, m, s), (22, 18, 30))
/// ```
pub const fn unix_clock(seconds_after_unix_epoch: u64) -> u32 {
    (seconds_after_unix_epoch % 86400) as _
}

#[inline]
/// Convert seconds to `hours`, `minutes` and `seconds`.
///
/// - The seconds returned is guaranteed to be `0..=59`
/// - The minutes returned is guaranteed to be `0..=59`
/// - The hours returned can be over `23`, as this is not a clock function,
///   see [`secs_to_clock`] for clock-like behavior that wraps around on `24`
///
/// ```rust
/// # use cuprate_helper::time::*;
/// // 59 seconds.
/// assert_eq!(secs_to_hms(59), (0, 0, 59));
///
/// // 1 minute.
/// assert_eq!(secs_to_hms(60), (0, 1, 0));
///
/// // 59 minutes, 59 seconds.
/// assert_eq!(secs_to_hms(3599), (0, 59, 59));
///
/// // 1 hour.
/// assert_eq!(secs_to_hms(3600), (1, 0, 0));
///
/// // 23 hours, 59 minutes, 59 seconds.
/// assert_eq!(secs_to_hms(86399), (23, 59, 59));
///
/// // 24 hours.
/// assert_eq!(secs_to_hms(86400), (24, 0, 0));
/// ```
pub const fn secs_to_hms(seconds: u64) -> (u64, u8, u8) {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = (seconds % 3600) % 60;

    debug_assert!(minutes < 60);
    debug_assert!(seconds < 60);

    (hours, minutes as u8, seconds as u8)
}

#[inline]
/// Convert seconds to clock time, `hours`, `minutes` and `seconds`.
///
/// This is the same as [`secs_to_hms`] except it will wrap around,
/// e.g, `24:00:00` would turn into `00:00:00`.
///
/// - The seconds returned is guaranteed to be `0..=59`
/// - The minutes returned is guaranteed to be `0..=59`
/// - The hours returned is guaranteed to be `0..=23`
///
/// ```rust
/// # use cuprate_helper::time::*;
/// // 59 seconds.
/// assert_eq!(secs_to_clock(59), (0, 0, 59));
///
/// // 1 minute.
/// assert_eq!(secs_to_clock(60), (0, 1, 0));
///
/// // 59 minutes, 59 seconds.
/// assert_eq!(secs_to_clock(3599), (0, 59, 59));
///
/// // 1 hour.
/// assert_eq!(secs_to_clock(3600), (1, 0, 0));
///
/// // 23 hours, 59 minutes, 59 seconds.
/// assert_eq!(secs_to_clock(86399), (23, 59, 59));
///
/// // 24 hours (wraps back)
/// assert_eq!(secs_to_clock(86400), (0, 0, 0));
///
/// // 24 hours, 59 minutes, 59 seconds (wraps back)
/// assert_eq!(secs_to_clock(89999), (0, 59, 59));
/// ```
pub const fn secs_to_clock(seconds: u32) -> (u8, u8, u8) {
    let seconds = seconds % 86400;
    let (h, m, s) = secs_to_hms(seconds as u64);

    debug_assert!(h < 24);
    debug_assert!(m < 60);
    debug_assert!(s < 60);

    #[allow(clippy::cast_possible_truncation)] // checked above
    (h as u8, m, s)
}

#[inline]
/// Get the current system time in the system's timezone
///
/// The returned value is the total amount of seconds passed in the current day.
///
/// This is guaranteed to return a value between `0..=86399`
///
/// This will return `0` if the underlying system call fails.
pub fn time() -> u32 {
    use chrono::Timelike;
    let now = chrono::offset::Local::now().time();
    (now.hour() * 3600) + (now.minute() * 60) + now.second()
}

#[inline]
/// Get the current system time in the UTC timezone
///
/// The returned value is the total amount of seconds passed in the current day.
///
/// This is guaranteed to return a value between `0..=86399`
pub fn time_utc() -> u32 {
    #[allow(clippy::cast_sign_loss)] // checked in function calls
    unix_clock(chrono::offset::Local::now().timestamp() as u64)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
