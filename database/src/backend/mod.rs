//! Database backends.
//!
//! TODO:
//! Create a test backend backed by `std::collections::HashMap`.
//!
//! The full type could be something like `HashMap<&'static str, HashMap<K, V>>`.
//! where the `str` is the table name, and the containing hashmap are are the
//! key and values.
//!
//! Not sure how duplicate keys will work.

cfg_if::cfg_if! {
    // If both backends are enabled, fallback to `heed`.
    // This is useful when using `--all-features`.
    if #[cfg(all(feature = "mdbx", not(feature = "heed")))] {
        mod mdbx;
        pub use mdbx::ConcreteEnv;
    } else {
        mod heed;
        pub use heed::ConcreteEnv;
    }
}
