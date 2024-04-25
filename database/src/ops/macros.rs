//! Macros.
//!
//! These generate repetitive documentation
//! for all the functions defined in `ops/`.

//---------------------------------------------------------------------------------------------------- Documentation macros
/// Generate documentation for the required `# Error` section.
macro_rules! doc_error {
    () => {
        r#"# Errors
This function returns [`RuntimeError::KeyNotFound`] if the input doesn't exist or other `RuntimeError`'s on database errors."#
    };
}
pub(super) use doc_error;

/// Generate `# Invariant` documentation for internal `fn`'s
/// that should be called directly with caution.
macro_rules! doc_add_block_inner_invariant {
    () => {
            r#"# ⚠️ Invariant ⚠️
This function mainly exists to be used internally by the parent function [`crate::ops::block::add_block`].

`add_block()` makes sure all data related to the input is mutated, while
this function _does not_, it specifically mutates _particular_ tables.

This is usually undesired - although this function is still available to call directly.

When calling this function, ensure that either:
1. This effect (incomplete database mutation) is what is desired, or that...
2. ...the other tables will also be mutated to a correct state"#
    };
}
pub(super) use doc_add_block_inner_invariant;
