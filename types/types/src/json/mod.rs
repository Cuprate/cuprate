//! This module contains types mappings for other common types
//! to allow for easier JSON (de)serialization.
//!
//! The main types are:
//! - [`block::Block`]
//! - [`tx::Transaction`]
//!
//! Modules exist within this module as the JSON representation
//! of types sometimes differs, thus, the modules hold the types
//! that match the specific schema, for example [`block::Input`]
//! is different than [`tx::Input`].

pub mod block;
pub mod output;
pub mod tx;
