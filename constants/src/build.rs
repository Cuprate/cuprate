//! Build related metadata.

/// The current commit hash of the root Cuprate repository.
///
/// # Case & length
/// It is guaranteed that `COMMIT` will be:
/// - Lowercase ASCII
/// - 40 characters long (no newline)
///
/// ```rust
/// # use cuprate_constants::build::*;
/// assert!(COMMIT.is_ascii());
/// assert_eq!(COMMIT.len(), 40);
/// assert_eq!(COMMIT.to_lowercase(), COMMIT);
/// ```
pub const COMMIT: &str = core::env!("COMMIT"); // Set in `constants/build.rs`.

/// `debug` if debug build, else `release`.
pub const BUILD: &str = if cfg!(debug_assertions) {
    "debug"
} else {
    "release"
};

/// `true` if debug build, else `false`.
pub const DEBUG: bool = cfg!(debug_assertions);

/// `true` if release build, else `false`.
pub const RELEASE: bool = !DEBUG;
