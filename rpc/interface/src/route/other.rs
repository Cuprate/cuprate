//! JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use std::{future::Future, sync::Arc};

use axum::{extract::State, http::StatusCode, Json};
use tower::{Service, ServiceExt};

use cuprate_rpc_types::{
    other::{OtherRequest, OtherResponse},
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
        if $handler.state().restricted() && $request.is_restricted() {
            return Err(StatusCode::NOT_FOUND);
        }
    };
}

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub(crate) async fn get_height<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_transactions<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_alt_blocks_hashes<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn is_key_image_spent<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn send_raw_transaction<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn start_mining<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn stop_mining<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn mining_status<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn save_bc<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_peer_list<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_public_nodes<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn set_log_hash_rate<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn set_log_level<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn set_log_categories<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_transaction_pool<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_transaction_pool_hashes<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_transaction_pool_stats<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn set_bootstrap_daemon<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn stop_daemon<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_info<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_net_stats<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_limit<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn set_limit<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn out_peers<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn in_peers<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn get_outs<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn update<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn pop_blocks<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    return_if_restricted!(handler, request);

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
