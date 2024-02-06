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
impl tower::Service<ReadRequest> for DatabaseService {
    type Response = ReadResponse;
    type Error = RuntimeError; // TODO: This could be a more specific error?
    type Future = std::future::Ready<Result<ReadResponse, RuntimeError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: ReadRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- `tower::Service` Writers
impl tower::Service<WriteRequest> for DatabaseService {
    type Response = WriteResponse;
    type Error = RuntimeError; // TODO: This could be a more specific error?
    type Future = std::future::Ready<Result<WriteResponse, RuntimeError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: WriteRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
