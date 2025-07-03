//! Database backends.

mod redb;
pub use redb::ConcreteEnv;

#[cfg(test)]
mod tests;
