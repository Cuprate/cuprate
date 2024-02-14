//! General `const`ants and `static`s.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
/// The current commit hash of the root Cuprate repository.
///
/// # Case & length
/// It is guaranteed that `COMMIT` will be:
/// - Lowercase
/// - 40 characters long (no newline)
///
/// ```rust
/// # use cuprate_helper::constants::*;
/// assert_eq!(COMMIT.as_bytes().len(), 40);
/// assert_eq!(COMMIT.to_lowercase(), COMMIT);
/// ```
pub const COMMIT: &str = core::env!("COMMIT"); // Set in `helper/build.rs`.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
