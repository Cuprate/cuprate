//! Read/write `Response`'s from the database.
//!
//! TODO: could add `strum` derives.

//---------------------------------------------------------------------------------------------------- Import
use crate::service::read::DatabaseReaderReceivers;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Response
#[derive(Debug)]
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

    /// TODO
    Shutdown(DatabaseReaderReceivers),
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
