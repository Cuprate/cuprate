//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, marker::PhantomData, sync::Arc};

use axum::extract::State;
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{error::Error, request::Request, response::Response, RpcService};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub trait RpcHandler: RpcService {
    /// TODO
    fn restricted(&self) -> bool;
}

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConcreteRpcHandler {
    restricted: bool,
}

impl RpcHandler for ConcreteRpcHandler {
    fn restricted(&self) -> bool {
        self.restricted
    }
}

impl Service<Request> for ConcreteRpcHandler {
    type Response = Response;
    type Error = Error;
    type Future = InfallibleOneshotReceiver<Result<Response, Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        todo!()
    }
}
