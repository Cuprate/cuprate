#![doc = include_str!("../README.md")]

pub mod config;
mod free;
pub mod ops;
#[cfg(feature = "service")]
pub mod service;
pub mod tables;
pub mod types;

pub use config::Config;
pub use free::open;

//re-exports
pub use cuprate_database;
