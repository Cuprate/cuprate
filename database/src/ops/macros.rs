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
