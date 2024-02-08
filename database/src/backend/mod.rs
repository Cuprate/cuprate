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
    if #[cfg(feature = "sanakirja")] {
        mod sanakirja;
        pub use sanakirja::ConcreteEnv;
        /// Static string of the `crate` being used as the database backend.
        pub const BACKEND: &str = "sanakirja";
    } else {
        mod heed;
        pub use heed::ConcreteEnv;
        /// Static string of the `crate` being used as the database backend.
        pub const BACKEND: &str = "heed";
    }
}
