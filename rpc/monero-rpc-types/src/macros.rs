//! Macros.
//!
//! These generate repetitive documentation, tests, etc.

//---------------------------------------------------------------------------------------------------- Documentation macros
/// TODO
macro_rules! serde_doc_test {
    (
        $type:ty, // TODO
    ) => {
        #[doc = "TODO"]
    };
}

/// TODO
macro_rules! monero_ref {
    (
        $monero_code_link:literal,    // TODO
        $monero_rpc_doc_link:literal, // TODO
    ) => {
        #[doc = "TODO"]
    };
}
