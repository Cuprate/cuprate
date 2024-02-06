//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{ReadRequest, ReadResponse, Readers, WriteRequest, WriteResponse, Writers},
    ConcreteDatabase,
};

use std::task::{Context, Poll};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- DatabaseService
/// TODO: maybe `CuprateDatabase`?
///
/// This struct represents the "owner" of the underlying database.
///
/// It handles all threads and is the 1 object other Cuprate
/// crates will send and receive requests & responses from.
#[allow(dead_code)] // TODO
pub struct DatabaseService {
    /// TODO: either `Arc` or `&'static` after `Box::leak`
    db: ConcreteDatabase,

    /// TODO
    readers: Readers,

    /// TODO
    writers: Writers,
}

//---------------------------------------------------------------------------------------------------- DatabaseService Impl
impl DatabaseService {
    /// TODO
    pub fn init(db: ConcreteDatabase) -> Self {
        #[allow(unused_variables)] // TODO
        let this = Self {
            db,
            readers: Readers::init(),
            writers: Writers::init(),
        };

        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- `tower::Service` Readers

//---------------------------------------------------------------------------------------------------- `tower::Service` Writers

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
