//! Binary route functions.

//---------------------------------------------------------------------------------------------------- Import
use axum::{body::Bytes, extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

use cuprate_epee_encoding::from_bytes;
use cuprate_rpc_types::{
    bin::{
        BinRequest, BinResponse, GetBlocksByHeightRequest, GetBlocksRequest, GetHashesRequest,
        GetOutputIndexesRequest, GetOutsRequest,
    },
    json::GetOutputDistributionRequest,
    RpcCall,
};

use crate::rpc_handler::RpcHandler;

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

/// De-duplicated inner function body for:
/// - [`generate_endpoints_with_input`]
/// - [`generate_endpoints_with_no_input`]
macro_rules! generate_endpoints_inner {
    ($variant:ident, $handler:ident, $request:expr_2021) => {
        paste::paste! {
            {
                // Check if restricted.
                //
                // INVARIANT:
                // The RPC handler functions in `cuprated` depend on this line existing,
                // the functions themselves do not check if they are being called
                // from an (un)restricted context. This line must be here or all
                // methods will be allowed to be called freely.
                if [<$variant Request>]::IS_RESTRICTED && $handler.is_restricted() {
                    // TODO: mimic `monerod` behavior.
                    return Err(StatusCode::FORBIDDEN);
                }

                // Send request.
                let Ok(response) = $handler.oneshot($request).await else {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                };

                let BinResponse::$variant(response) = response else {
                    panic!("RPC handler returned incorrect response");
                };

                // Serialize to bytes and respond.
                match cuprate_epee_encoding::to_bytes(response) {
                    Ok(bytes) => Ok(bytes.freeze()),
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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

#[ derive(Serialize, Deserialize)]
pub(crate) struct Null {}

pub(crate) async fn get_transaction_pool_hashes<H: RpcHandler>(
    State(handler): State<H>,
    Json(Null {}): Json<Null>,
) -> Result<String, StatusCode> {
    Ok(format!(
        r#"{{
    "credits": 0,
    "status": "OK",
    "top_hash": "",
    "untrusted": false
    }}
    "#
    ))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
