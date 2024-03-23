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

//---------------------------------------------------------------------------------------------------- Response
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A response from the database.
///
/// TODO
pub enum Response {
    //-------------------------------------------------------- Read responses
    /// TODO
    Example1,
    /// TODO
    Example2(usize),
    /// TODO
    Example3(String),

    //-------------------------------------------------------- Write responses
    /// The response
    ///
    /// TODO
    ExampleWriteResponse, // Probably will be just `Ok`
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
