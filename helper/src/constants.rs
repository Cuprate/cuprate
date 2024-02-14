//! General `const`ants and `static`s.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
/// The current commit hash of the root Cuprate repository.
///
/// ```rust
/// # use cuprate_helper::constants::*;
/// // Commit hash is always 40 characters long.
/// assert_eq!(COMMIT.len(), 40);
/// ```
pub const COMMIT: &str = core::env!("COMMIT"); // Set in `helper/build.rs`.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
