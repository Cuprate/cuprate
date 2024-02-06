//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    service::{Readers, Writer},
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
    /// TODO
    db: ConcreteDatabase, // TODO: either `Arc` or `&'static` after `Box::leak`

    /// TODO
    readers: Readers,

    /// TODO
    writer: Writer,
}

//---------------------------------------------------------------------------------------------------- DatabaseService Impl
impl DatabaseService {
    /// TODO
    pub fn new(db: ConcreteDatabase) -> Self {
        #[allow(unused_variables)]
        let this = Self {
            db,
            readers: Readers,
            writer: Writer,
        };

        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Trait Impl
/// TODO - temporary struct for gist.
pub struct Request;
/// TODO - temporary struct for gist.
pub struct Response;
/// TODO - temporary struct for gist.
pub struct Error;

impl tower::Service<Request> for DatabaseService {
    type Response = Response;
    type Error = Error;
    type Future = std::future::Ready<Result<Response, Error>>;

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
