//! Macros.
//!
//! These generate repetitive documentation
//! for all the functions defined in `ops/`.

//---------------------------------------------------------------------------------------------------- Documentation macros
/// Generate documentation for the required `# Error` section.
macro_rules! doc_error {
    // Single use function, e.g., `get_block()`
    () => {
        r#"# Errors
This function returns [`RuntimeError::KeyNotFound`] if the input doesn't exist or other `RuntimeError`'s on database errors."#
    };

    // Bulk use function, e.g., `get_block_bulk()`
    (bulk) => {
        r#"# Errors
This function returns [`RuntimeError::KeyNotFound`] if the input doesn't exist or other `RuntimeError`'s on database errors.

Note that this function will early return if any individual operation returns an error - all operations are either OK or not."#
    };
}
pub(super) use doc_error;

/// Generate documentation for either single functions or their `_bulk()` versions.
///
/// # Usage
/// For single functions -> `#[doc = doc_fn!($name_of_bulk_fn)]`
/// For bulk functions -> `#[doc = doc_fn!($name_of_single_fn, bulk)]`
macro_rules! doc_fn {
    // For bulk functions.
    (
        $single_fn:ident, bulk // `fn` name of the single function to link to.
    ) => {
        concat!(
            "Bulk version of [`",
            stringify!($single_fn),
            r#"()`].

This function operates on bulk input more efficiently than the above function.

See `"#,
            stringify!($single_fn),
            "()` for more documentation.",
        )
    };

    (
        $bulk_fn:ident // `fn` name of the bulk function to link to.
    ) => {
        concat!(
            "Consider using [`",
            stringify!($bulk_fn),
            "()`] for multiple inputs."
        )
    };
}
pub(super) use doc_fn;

/// Generate documentation for `_inner()` functions.
///
/// See this modules `mod.rs` for info.
macro_rules! doc_inner {
    (
        $fn:ident // `fn` name of the parent function to link to.
    ) => {
        concat!(
            "Internal function for [`",
            stringify!($fn),
            r#"()`].

This function is the _internal_ implementation of the above function (and the `_bulk()` version if it exists).

In contrast to the above function, this function takes already opened [`DatabaseRo`]/[`DatabaseIter`]/[`DatabaseRw`]
types directly - as such, if you already have opened tables and would like to make multiple function calls, these `_inner()`
functions are the cheaper choice.

See `"#,
            stringify!($fn),
            r#"()` for more documentation on the actual function.

# Return
This function returns the same value as the actual function.

# Errors
This function errors in the same way as the actual function."#,
        )
    };
}
pub(super) use doc_inner;
