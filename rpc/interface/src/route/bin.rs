//! Binary types from [`.bin` endpoints](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin).
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use std::{future::Future, sync::Arc};

use axum::{extract::State, http::StatusCode, Json};
use tower::{Service, ServiceExt};

use crate::{
    error::Error, json_rpc_method::JsonRpcMethod, request::Request, response::Response,
    rpc_handler::RpcHandler, rpc_state::RpcState,
};

//---------------------------------------------------------------------------------------------------- Macro
/// TODO
macro_rules! return_if_restricted {
    ($handler:ident) => {
        if $handler.state().restricted() {
            return Ok("TODO");
        }
    };
}

//---------------------------------------------------------------------------------------------------- Struct definitions
/// TODO
pub(crate) async fn get_blocks<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

/// TODO
pub(crate) async fn get_blocks_by_height<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

/// TODO
pub(crate) async fn get_hashes<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

/// TODO
pub(crate) async fn get_o_indexes<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

/// TODO
pub(crate) async fn get_outs<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

/// TODO
pub(crate) async fn get_transaction_pool_hashes<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

/// TODO
pub(crate) async fn get_output_distribution<H: RpcHandler>(
    State(handler): State<H>,
) -> Result<&'static str, StatusCode> {
    return_if_restricted!(handler);
    /* call handler */
    Ok("TODO")
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
