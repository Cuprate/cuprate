//! TODO

cfg_if::cfg_if! {
    if #[cfg(feature = "sanakirja")] {
        mod sanakirja;
        pub use sanakirja::Sanakirja as ConcreteDatabase;
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "sanakirja";
    } else {
        mod heed;
        pub use heed::Heed as ConcreteDatabase;
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "heed";
    }
}
