//! Read/write `Request`s to the database.
//!
//! TODO: could add `strum` derives.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- ReadRequest
#[derive(Debug)]
/// A read request to the database.
pub enum ReadRequest {
    /// TODO
    Example1,
    /// TODO
    Example2(usize),
    /// TODO
    Example3(String),
    /// TODO
    Shutdown,
}

//---------------------------------------------------------------------------------------------------- WriteRequest
#[derive(Debug)]
/// A write request to the database.
pub enum WriteRequest {
    /// TODO
    Example1,
    /// TODO
    Example2(usize),
    /// TODO
    Example3(String),
    ///
    Shutdown,
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
