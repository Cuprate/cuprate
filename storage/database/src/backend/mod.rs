//! Database backends.

#[cfg(feature = "heed")]
mod heed;

#[cfg(feature = "heed")]
pub use heed::ConcreteEnv as HeedEnv;

#[cfg(feature = "redb")]
mod redb;

#[cfg(feature = "redb")]
pub use redb::ConcreteEnv as RedbEnv;

cfg_if::cfg_if! {
    // TODO remove this block when all ConcreteEnv references are gone.
    if #[cfg(all(feature = "redb", not(feature = "heed")))] {
        pub use redb::ConcreteEnv;
    } else {
        pub use heed::ConcreteEnv;
    }
}

#[cfg(test)]
mod tests;
