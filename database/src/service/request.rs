//! Read/write `Request`s to the database.
//!
//! TODO: could add `strum` derives.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- ReadRequest
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A read request to the database.
pub enum ReadRequest {
    /// TODO
    Example1,
    /// TODO
    Example2(usize),
    /// TODO
    Example3(String),
}

//---------------------------------------------------------------------------------------------------- WriteRequest
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A write request to the database.
pub enum WriteRequest {
    /// TODO
    Example1,
    /// TODO
    Example2(usize),
    /// TODO
    Example3(String),
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
