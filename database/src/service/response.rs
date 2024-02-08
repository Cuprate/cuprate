//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Response
#[derive(Debug)]
/// Either a read or write response.
///
/// TODO: not sure if we actually need this.
pub enum Response {
    /// TODO
    Read(ReadResponse),
    /// TODO
    Write(WriteResponse),
}

//---------------------------------------------------------------------------------------------------- ReadResponse
#[derive(Debug)]
/// A read response from the database.
pub enum ReadResponse {
    /// TODO
    Example1,
    /// TODO
    Example2(usize),
    /// TODO
    Example3(String),
}

//---------------------------------------------------------------------------------------------------- WriteResponse
#[derive(Debug)]
/// A write response from the database.
pub enum WriteResponse {
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
