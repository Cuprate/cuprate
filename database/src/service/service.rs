//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    service::{Readers, Request, Response, Writer},
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
    writer: Writer,
}

//---------------------------------------------------------------------------------------------------- DatabaseService Impl
impl DatabaseService {
    /// TODO
    pub fn new(db: ConcreteDatabase) -> Self {
        #[allow(unused_variables)] // TODO
        let this = Self {
            db,
            readers: Readers,
            writer: Writer,
        };

        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- `tower::Service` Impl
impl tower::Service<Request> for DatabaseService {
    type Response = Response;
    type Error = RuntimeError;
    type Future = std::future::Ready<Result<Response, RuntimeError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _req: Request) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
