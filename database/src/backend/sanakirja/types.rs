//! `sanakirja` type aliases.

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `sanakirja`.
pub(super) type SanakirjaDb =
    sanakirja::btree::Db_<[u8], [u8], sanakirja::btree::page_unsized::Page<[u8], [u8]>>;
