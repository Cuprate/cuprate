//! Database backends.

cfg_if::cfg_if! {
    // If both backends are enabled, fallback to `heed`.
    // This is useful when using `--all-features`.
    if #[cfg(all(feature = "redb", not(feature = "heed")))] {
        use heed as _;

        mod redb;
        pub use redb::ConcreteEnv;
    } else {
        mod heed;
        pub use heed::ConcreteEnv;
    }
}

#[cfg(test)]
mod tests;
