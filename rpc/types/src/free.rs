//! Free functions.

//---------------------------------------------------------------------------------------------------- Serde
// These are functions used for conditionally (de)serialization.

/// Returns `true` if the input `u` is equal to `0`.
#[inline]
#[allow(clippy::trivially_copy_pass_by_ref)] // serde needs `&`
#[allow(dead_code)] // TODO: see if needed after handlers.
pub(crate) const fn is_zero(u: &u64) -> bool {
    *u == 0
}

/// Returns `true` the input `u` is equal to `1`.
#[inline]
#[allow(clippy::trivially_copy_pass_by_ref)] // serde needs `&`
#[allow(dead_code)] // TODO: see if needed after handlers.
pub(crate) const fn is_one(u: &u64) -> bool {
    *u == 1
}
