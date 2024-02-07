//! Database backends.

cfg_if::cfg_if! {
    if #[cfg(feature = "sanakirja")] {
        mod sanakirja;
        pub use sanakirja::ConcreteDatabase;
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "sanakirja";
    } else {
        mod heed;
        pub use heed::ConcreteDatabase;
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "heed";
    }
}
