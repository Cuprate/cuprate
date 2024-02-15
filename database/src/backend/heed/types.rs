//! `heed` type aliases.

//---------------------------------------------------------------------------------------------------- Use
use heed::{types::Bytes, Database};

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `heed`.
pub(super) type HeedDb = Database<Bytes, Bytes>;
