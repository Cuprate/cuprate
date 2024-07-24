//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, future::Future, sync::Arc};

use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse, Json};
use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id,
};
use tower::{Service, ServiceExt};

use cuprate_epee_encoding::from_bytes;
use cuprate_rpc_types::{
    bin::{
        BinRequest, BinResponse, GetBlocksByHeightRequest, GetBlocksByHeightResponse,
        GetBlocksRequest, GetBlocksResponse, GetHashesRequest, GetHashesResponse,
        GetOutputIndexesRequest, GetOutputIndexesResponse, GetOutsRequest, GetOutsResponse,
        GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
    },
    json::{
        GetOutputDistributionRequest, GetOutputDistributionResponse, JsonRpcRequest,
        JsonRpcResponse,
    },
    other::{OtherRequest, OtherResponse},
    RpcRequest,
};

use crate::{
    error::Error, request::Request, response::Response, rpc_handler::RpcHandler,
    rpc_state::RpcState,
};

//---------------------------------------------------------------------------------------------------- Routes
/// TODO
macro_rules! generate_endpoints {
    ($(
        $endpoint:ident => $variant:ident
    ),*) => { paste::paste! {
        $(
            /// TODO
            #[allow(unused_mut)]
            pub(crate) async fn $endpoint<H: RpcHandler>(
                State(handler): State<H>,
                mut request: Bytes,
            ) -> Result<Bytes, StatusCode> {
                // Serialize into the request type.
                let request = BinRequest::$variant(
                    from_bytes(&mut request).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                );

                // TODO: call handler
                let Response::Binary(response) = todo!() else {
                    panic!("RPC handler did not return a binary response");
                };

                // Assert the response from the inner handler is correct.
                let BinResponse::$variant(response) = response else {
                    panic!("RPC handler returned incorrect response");
                };

                // Serialize to bytes and respond.
                match cuprate_epee_encoding::to_bytes(response) {
                    Ok(bytes) => Ok(bytes.freeze()),
                    Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        )*
    }};
}

generate_endpoints! {
    get_blocks => GetBlocks,
    get_blocks_by_height => GetBlocksByHeight,
    get_hashes => GetHashes,
    get_o_indexes => GetOutputIndexes,
    get_outs => GetOuts,
    get_transaction_pool_hashes => GetTransactionPoolHashes,
    get_output_distribution => GetOutputDistribution
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
