//! `heed` type aliases.

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `heed`.
pub(super) type HeedDb =
    heed::Database<heed::types::Bytes, heed::types::Bytes>;
