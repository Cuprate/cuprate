//! TODO

cfg_if::cfg_if! {
    if #[cfg(feature = "sanakirja")] {
        mod sanakirja;
        pub use sanakirja::Sanakirja as ConcreteDatabase;
    } else {
        mod heed;
        pub use heed::Heed as ConcreteDatabase;
    }
}
