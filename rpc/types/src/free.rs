//! Macros.

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[allow(clippy::trivially_copy_pass_by_ref)] // serde needs `&`
pub(crate) const fn is_zero(u: &u64) -> bool {
    *u == 0
}

/// TODO
#[allow(clippy::trivially_copy_pass_by_ref)] // serde needs `&`
pub(crate) const fn is_one(u: &u64) -> bool {
    *u == 1
}
