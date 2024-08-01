//! Binary route functions.
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use axum::{body::Bytes, extract::State, http::StatusCode};
use tower::ServiceExt;

use cuprate_epee_encoding::from_bytes;
use cuprate_rpc_types::bin::{BinRequest, BinResponse, GetTransactionPoolHashesRequest};

use crate::{rpc_handler::RpcHandler, rpc_request::RpcRequest, rpc_response::RpcResponse};

//---------------------------------------------------------------------------------------------------- Routes
/// TODO
macro_rules! generate_endpoints_with_input {
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

                generate_endpoints_inner!($variant, handler, request)
            }
        )*
    }};
}

/// TODO
macro_rules! generate_endpoints_with_no_input {
    ($(
        $endpoint:ident => $variant:ident
    ),*) => { paste::paste! {
        $(
            /// TODO
            #[allow(unused_mut)]
            pub(crate) async fn $endpoint<H: RpcHandler>(
                State(handler): State<H>,
            ) -> Result<Bytes, StatusCode> {
                const REQUEST: BinRequest = BinRequest::$variant([<$variant Request>] {});
                generate_endpoints_inner!($variant, handler, REQUEST)
            }
        )*
    }};
}

/// TODO
macro_rules! generate_endpoints_inner {
    ($variant:ident, $handler:ident, $request:expr) => {
        paste::paste! {
            {
                // Send request.
                let request = RpcRequest::Binary($request);
                let channel = $handler.oneshot(request).await?;

                // Assert the response from the inner handler is correct.
                let RpcResponse::Binary(response) = todo!() else {
                    panic!("RPC handler did not return a binary response");
                };
                let BinResponse::$variant(response) = response else {
                    panic!("RPC handler returned incorrect response");
                };

                // Serialize to bytes and respond.
                match cuprate_epee_encoding::to_bytes(response) {
                    Ok(bytes) => Ok(bytes.freeze()),
                    Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        }
    };
}

generate_endpoints_with_input! {
    get_blocks => GetBlocks,
    get_blocks_by_height => GetBlocksByHeight,
    get_hashes => GetHashes,
    get_o_indexes => GetOutputIndexes,
    get_outs => GetOuts,
    get_output_distribution => GetOutputDistribution
}

generate_endpoints_with_no_input! {
    get_transaction_pool_hashes => GetTransactionPoolHashes
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}