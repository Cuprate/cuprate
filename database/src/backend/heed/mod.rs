//! `heed` backend implementation.

mod database;
pub use database::ConcreteDatabase;

mod serde;
mod transaction;
