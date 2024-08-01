//! Database backends.

#[cfg(feature = "heed")]
mod heed;

#[cfg(feature = "heed")]
pub use heed::ConcreteEnv as HeedEnv;

#[cfg(feature = "redb")]
mod redb;

#[cfg(feature = "redb")]
pub use redb::ConcreteEnv as RedbEnv;

#[cfg(test)]
mod tests;
