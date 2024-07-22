//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, marker::PhantomData, sync::Arc};

use tower::Service;

use crate::{error::Error, request::Request, response::Response};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub trait RpcState
where
    Self: Clone + Send + Sync + 'static,
{
    /// TODO
    fn restricted(&self) -> bool;
}

/// TODO
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConcreteRpcState {
    restricted: bool,
}

impl RpcState for ConcreteRpcState {
    fn restricted(&self) -> bool {
        self.restricted
    }
}
