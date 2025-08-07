//! Database backends.
// TODO cleanup before merging #510

#[cfg(feature = "redb")]
mod redb;

#[cfg(feature = "heed")]
mod heed;

cfg_if::cfg_if! {
    // If both backends are enabled, fallback to `heed`.
    // This is useful when using `--all-features`.
    if #[cfg(all(feature = "redb", not(feature = "heed")))] {
        pub use redb::ConcreteEnv;
        pub use redb::ConcreteEnv as ReedEnv;
    } else {
        pub use heed::ConcreteEnv;
        pub use heed::ConcreteEnv as HeedEnv;
    }
}

#[cfg(all(feature = "heed", feature = "redb"))]
pub use redb::ConcreteEnv as RedbEnv;

#[cfg(test)]
mod tests;
