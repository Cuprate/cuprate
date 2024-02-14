//! General `const`ants and `static`s.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
/// The current commit hash of the root Cuprate repository.
///
/// ```rust
/// # use cuprate_helper::constants::*;
/// // Commit hash is always 40 bytes long
/// // (but not necessarily 40 ASCII characters).
/// assert_eq!(COMMIT.as_bytes().len(), 40);
/// ```
pub const COMMIT: &str = core::env!("COMMIT"); // Set in `helper/build.rs`.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
