//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, marker::PhantomData, sync::Arc};

use axum::extract::State;
use tower::Service;

use crate::{
    error::Error, request::Request, response::Response, rpc_state::ConcreteRpcState, RpcState,
};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub trait RpcHandler: Clone + Send + Sync + 'static {
    /// TODO
    type State: RpcState;

    /// TODO
    type Handler: Send + Sync + 'static + Service<Request>;
    // where
    //     <Self::Handler as Service<Request>>::Response: Into<Response>,
    //     <Self::Handler as Service<Request>>::Error: Into<Error>,
    //     <Self::Handler as Service<Request>>::Future: Future<Output = Result<Response, Error>>;

    /// TODO
    fn state(&self) -> &Self::State;

    /// TODO
    fn handler(&self) -> Self::Handler;
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConcreteRpcHandler<Handler> {
    state: ConcreteRpcState,
    _handler: PhantomData<Handler>,
}

impl<H> RpcHandler for ConcreteRpcHandler<H>
where
    H: Clone + Send + Sync + 'static + Service<Request>,
    <H as Service<Request>>::Response: Into<Response>,
    <H as Service<Request>>::Error: Into<Error>,
    <H as Service<Request>>::Future: Future<Output = Result<Response, Error>>,
{
    type State = ConcreteRpcState;
    type Handler = H;

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn handler(&self) -> Self::Handler {
        todo!()
    }
}
