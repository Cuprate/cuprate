//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, future::Future, sync::Arc};

use axum::{body::Bytes, extract::State, http::StatusCode, Json};
use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id,
};
use tower::{Service, ServiceExt};

use cuprate_rpc_types::{
    other::{
        GetAltBlocksHashesRequest, GetAltBlocksHashesResponse, GetHeightRequest, GetHeightResponse,
        GetLimitRequest, GetLimitResponse, GetNetStatsRequest, GetNetStatsResponse, GetOutsRequest,
        GetOutsResponse, GetPeerListRequest, GetPeerListResponse, GetPublicNodesRequest,
        GetPublicNodesResponse, GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
        GetTransactionPoolRequest, GetTransactionPoolResponse, GetTransactionPoolStatsRequest,
        GetTransactionPoolStatsResponse, GetTransactionsRequest, GetTransactionsResponse,
        GetTxIdsLooseRequest, GetTxIdsLooseResponse, InPeersRequest, InPeersResponse,
        IsKeyImageSpentRequest, IsKeyImageSpentResponse, MiningStatusRequest, MiningStatusResponse,
        OtherRequest, OtherResponse, OutPeersRequest, OutPeersResponse, PopBlocksRequest,
        PopBlocksResponse, SaveBcRequest, SaveBcResponse, SendRawTransactionRequest,
        SendRawTransactionResponse, SetBootstrapDaemonRequest, SetBootstrapDaemonResponse,
        SetLimitRequest, SetLimitResponse, SetLogCategoriesRequest, SetLogCategoriesResponse,
        SetLogHashRateRequest, SetLogHashRateResponse, SetLogLevelRequest, SetLogLevelResponse,
        StartMiningRequest, StartMiningResponse, StopDaemonRequest, StopDaemonResponse,
        StopMiningRequest, StopMiningResponse, UpdateRequest, UpdateResponse,
    },
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
        $endpoint:ident => $variant:ident $(=> $constructor:expr)?
    ),*) => { paste::paste! {
        $(
            /// TODO
            #[allow(unused_mut)]
            pub(crate) async fn $endpoint<H: RpcHandler>(
                State(handler): State<H>,
                Json(request): Json<[<$variant Request>]>,
            ) -> Result<Json<[<$variant Response>]>, StatusCode> {
                if request.is_restricted() && handler.state().restricted() {
                    todo!();
                }

                // TODO: call handler
                let Response::Other(response) = todo!() else {
                    panic!("RPC handler did not return a binary response");
                };

                // Assert the response from the inner handler is correct.
                match response {
                    OtherResponse::$variant(response) => Ok(Json(response)),
                    _ => panic!("RPC handler returned incorrect response"),
                }
            }
        )*
    }};
}

generate_endpoints! {
    get_height => GetHeight,
    get_transactions => GetTransactions,
    get_alt_blocks_hashes => GetAltBlocksHashes,
    is_key_image_spent => IsKeyImageSpent,
    send_raw_transaction => SendRawTransaction,
    start_mining => StartMining,
    stop_mining => StopMining,
    mining_status => MiningStatus,
    save_bc => SaveBc,
    get_peer_list => GetPeerList,
    set_log_hash_rate => SetLogHashRate,
    set_log_level => SetLogLevel,
    set_log_categories => SetLogCategories,
    set_bootstrap_daemon => SetBootstrapDaemon,
    get_transaction_pool => GetTransactionPool,
    get_transaction_pool_stats => GetTransactionPoolStats,
    stop_daemon => StopDaemon,
    get_limit => GetLimit,
    set_limit => SetLimit,
    out_peers => OutPeers,
    in_peers => InPeers,
    get_net_stats => GetNetStats,
    get_outs => GetOuts,
    update => Update,
    pop_blocks => PopBlocks,
    get_transaction_ids_loose => GetTxIdsLoose,
    get_transaction_pool_hashes => GetTransactionPoolHashes,
    get_public_nodes => GetPublicNodes
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
