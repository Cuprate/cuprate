//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, marker::PhantomData, sync::Arc};

use axum::extract::State;
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{error::Error, request::Request, response::Response};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub trait RpcService:
    Clone
    + Send
    + Sync
    + 'static
    + Service<
        Request,
        Response = Response,
        Error = Error,
        Future = InfallibleOneshotReceiver<Result<Response, Error>>,
    >
{
}

/// TODO
impl<S> RpcService for S where
    Self: Clone
        + Send
        + Sync
        + 'static
        + Service<
            Request,
            Response = Response,
            Error = Error,
            Future = InfallibleOneshotReceiver<Result<Response, Error>>,
        >
{
}
