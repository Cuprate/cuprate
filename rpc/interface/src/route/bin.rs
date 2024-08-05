//! Binary route functions.

//---------------------------------------------------------------------------------------------------- Import
use axum::{body::Bytes, extract::State, http::StatusCode};
use tower::ServiceExt;

use cuprate_epee_encoding::from_bytes;
use cuprate_rpc_types::bin::{BinRequest, BinResponse, GetTransactionPoolHashesRequest};

use crate::{rpc_handler::RpcHandler, rpc_request::RpcRequest, rpc_response::RpcResponse};

//---------------------------------------------------------------------------------------------------- Routes
/// This macro generates route functions that expect input.
///
/// See below for usage.
macro_rules! generate_endpoints_with_input {
    ($(
        // Syntax:
        // Function name => Expected input type
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

/// This macro generates route functions that expect _no_ input.
///
/// See below for usage.
macro_rules! generate_endpoints_with_no_input {
    ($(
        // Syntax:
        // Function name => Expected input type (that is empty)
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

/// De-duplicated inner function body for:
/// - [`generate_endpoints_with_input`]
/// - [`generate_endpoints_with_no_input`]
macro_rules! generate_endpoints_inner {
    ($variant:ident, $handler:ident, $request:expr) => {
        paste::paste! {
            {
                // Send request.
                let request = RpcRequest::Binary($request);
                let channel = $handler.oneshot(request).await?;

                // Assert the response from the inner handler is correct.
                let RpcResponse::Binary(response) = channel else {
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
