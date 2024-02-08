//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Request
#[derive(Debug)]
/// TODO: not sure if we actually need this.
///
/// Either a read or write request.
pub enum Request {
    /// TODO
    Read(ReadRequest),
    /// TODO
    Write(WriteRequest),
}

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
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
