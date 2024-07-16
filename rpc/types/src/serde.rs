//! Custom (de)serialization functions for serde.

//---------------------------------------------------------------------------------------------------- Lints
#![allow(clippy::trivially_copy_pass_by_ref)] // serde fn signature

//---------------------------------------------------------------------------------------------------- Import
use serde::Serializer;

//---------------------------------------------------------------------------------------------------- Free functions
#[inline]
pub(crate) fn serde_true<S>(_: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(true)
}

#[inline]
pub(crate) fn serde_false<S>(_: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(false)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;
}
