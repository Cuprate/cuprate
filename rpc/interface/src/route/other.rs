//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use axum::{extract::State, http::StatusCode, Json};
use tower::ServiceExt;

use cuprate_rpc_types::{
    other::{
        GetAltBlocksHashesRequest, GetAltBlocksHashesResponse, GetHeightRequest, GetHeightResponse,
        GetLimitRequest, GetLimitResponse, GetNetStatsRequest, GetNetStatsResponse, GetOutsRequest,
        GetOutsResponse, GetPeerListRequest, GetPeerListResponse, GetPublicNodesRequest,
        GetPublicNodesResponse, GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
        GetTransactionPoolRequest, GetTransactionPoolResponse, GetTransactionPoolStatsRequest,
        GetTransactionPoolStatsResponse, GetTransactionsRequest, GetTransactionsResponse,
        InPeersRequest, InPeersResponse, IsKeyImageSpentRequest, IsKeyImageSpentResponse,
        MiningStatusRequest, MiningStatusResponse, OtherRequest, OtherResponse, OutPeersRequest,
        OutPeersResponse, PopBlocksRequest, PopBlocksResponse, SaveBcRequest, SaveBcResponse,
        SendRawTransactionRequest, SendRawTransactionResponse, SetBootstrapDaemonRequest,
        SetBootstrapDaemonResponse, SetLimitRequest, SetLimitResponse, SetLogCategoriesRequest,
        SetLogCategoriesResponse, SetLogHashRateRequest, SetLogHashRateResponse,
        SetLogLevelRequest, SetLogLevelResponse, StartMiningRequest, StartMiningResponse,
        StopDaemonRequest, StopDaemonResponse, StopMiningRequest, StopMiningResponse,
        UpdateRequest, UpdateResponse,
    },
    RpcCall,
};

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
                State(mut handler): State<H>,
                Json(request): Json<[<$variant Request>]>,
            ) -> Result<Json<[<$variant Response>]>, StatusCode> {
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
                State(mut handler): State<H>,
            ) -> Result<Json<[<$variant Response>]>, StatusCode> {
                generate_endpoints_inner!($variant, handler, [<$variant Request>] {})
            }
        )*
    }};
}

/// TODO
macro_rules! generate_endpoints_inner {
    ($variant:ident, $handler:ident, $request:expr) => {
        paste::paste! {
            {
                // Check if restricted.
                if [<$variant Request>]::IS_RESTRICTED && $handler.restricted() {
                    // TODO: mimic `monerod` behavior.
                    return Err(StatusCode::FORBIDDEN);
                }

                // Send request.
                let request = RpcRequest::Other(OtherRequest::$variant($request));
                let channel = $handler.oneshot(request).await?;

                // Assert the response from the inner handler is correct.
                let RpcResponse::Other(response) = channel else {
                    panic!("RPC handler did not return a binary response");
                };
                let OtherResponse::$variant(response) = response else {
                    panic!("RPC handler returned incorrect response")
                };

                Ok(Json(response))
            }
        }
    };
}

generate_endpoints_with_input! {
    get_transactions => GetTransactions,
    is_key_image_spent => IsKeyImageSpent,
    send_raw_transaction => SendRawTransaction,
    start_mining => StartMining,
    get_peer_list => GetPeerList,
    set_log_hash_rate => SetLogHashRate,
    set_log_level => SetLogLevel,
    set_log_categories => SetLogCategories,
    set_bootstrap_daemon => SetBootstrapDaemon,
    set_limit => SetLimit,
    out_peers => OutPeers,
    in_peers => InPeers,
    get_outs => GetOuts,
    update => Update,
    pop_blocks => PopBlocks,
    get_public_nodes => GetPublicNodes
}

generate_endpoints_with_no_input! {
    get_height => GetHeight,
    get_alt_blocks_hashes => GetAltBlocksHashes,
    stop_mining => StopMining,
    mining_status => MiningStatus,
    save_bc => SaveBc,
    get_transaction_pool => GetTransactionPool,
    get_transaction_pool_stats => GetTransactionPoolStats,
    stop_daemon => StopDaemon,
    get_limit => GetLimit,
    get_net_stats => GetNetStats,
    get_transaction_pool_hashes => GetTransactionPoolHashes
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
