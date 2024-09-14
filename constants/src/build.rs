//! TODO

/// The current commit hash of the root Cuprate repository.
///
/// # Case & length
/// It is guaranteed that `COMMIT` will be:
/// - Lowercase ASCII
/// - 40 characters long (no newline)
///
/// ```rust
/// # use cuprate_constants::build::*;
/// assert_eq!(COMMIT.is_ascii());
/// assert_eq!(COMMIT.as_bytes().len(), 40);
/// assert_eq!(COMMIT.to_lowercase(), COMMIT);
/// ```
pub const COMMIT: &str = core::env!("COMMIT"); // Set in `helper/build.rs`.

/// TODO
pub const DEBUG: bool = cfg!(debug_assertions);

/// TODO
pub const RELEASE: bool = !DEBUG;
