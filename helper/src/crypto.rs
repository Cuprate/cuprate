//! Crypto related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use curve25519_dalek::edwards::CompressedEdwardsY;

//---------------------------------------------------------------------------------------------------- Public API
#[inline]
/// Checks that a point is canonical.
///
/// https://github.com/dalek-cryptography/curve25519-dalek/issues/380
///
/// ```rust
/// # use helper::crypto::*;
/// # use curve25519_dalek::edwards::CompressedEdwardsY;
/// let slice = [1; 32];
/// let point = CompressedEdwardsY(slice);
/// assert!(check_point(&point));
/// ```
pub fn check_point(point: &CompressedEdwardsY) -> bool {
    let bytes = point.as_bytes();

    point
        .decompress()
        // Ban points which are either unreduced or -0
        .filter(|point| point.compress().as_bytes() == bytes)
        .is_some()
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
