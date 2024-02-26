//! `mdbx` type aliases.

//---------------------------------------------------------------------------------------------------- Types
/// The concrete database type for `mdbx`.
pub(super) type MdbxDb<'a> = libmdbx::Table<'a>;
