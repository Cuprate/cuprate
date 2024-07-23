//! Binary types from [`.bin` endpoints](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin).
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use std::{future::Future, sync::Arc};

use axum::{body::Bytes, extract::State, http::StatusCode, Json};
use tower::{Service, ServiceExt};

use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    RpcRequest,
};

use crate::{
    error::Error, request::Request, response::Response, rpc_handler::RpcHandler,
    rpc_state::RpcState,
};

//---------------------------------------------------------------------------------------------------- Macro
/// TODO
macro_rules! return_if_restricted {
    ($handler:ident, $request:ident) => {
        // TODO
        // if $handler.state().restricted() && $request.is_restricted() {
        if $handler.state().restricted() {
            return Err(StatusCode::NOT_FOUND);
        }
    };
}

//---------------------------------------------------------------------------------------------------- Struct definitions
/// TODO
pub(crate) async fn get_blocks<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes, // TODO: BinRequest
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

/// TODO
pub(crate) async fn get_blocks_by_height<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

/// TODO
pub(crate) async fn get_hashes<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

/// TODO
pub(crate) async fn get_o_indexes<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

/// TODO
pub(crate) async fn get_outs<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

/// TODO
pub(crate) async fn get_transaction_pool_hashes<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

/// TODO
pub(crate) async fn get_output_distribution<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes,
) -> Result<Vec<u8>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
