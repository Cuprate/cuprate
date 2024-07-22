//! Free functions.

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, marker::PhantomData};

use axum::{routing::method_routing::get, Router};
use tower::Service;

use crate::{
    error::Error, request::Request, response::Response, route::json_rpc, rpc_handler::RpcHandler,
    RpcState,
};

//---------------------------------------------------------------------------------------------------- Router
/// TODO
#[allow(clippy::needless_pass_by_value)]
pub fn create_router<H: RpcHandler>() -> Router<H::RpcState> {
    // List of `monerod` routes:
    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L97-L189>
    Router::new()
        // JSON-RPC route.
        .route("/json_rpc", get(json_rpc))
        // Other JSON routes.
        .route("/get_height", todo!())
        .route("/getheight", todo!())
        .route("/get_transactions", todo!())
        .route("/gettransactions", todo!())
        .route("/get_alt_blocks_hashes", todo!())
        .route("/is_key_image_spent", todo!())
        .route("/send_raw_transaction", todo!())
        .route("/sendrawtransaction", todo!())
        .route("/start_mining", todo!())
        .route("/stop_mining", todo!())
        .route("/mining_status", todo!())
        .route("/save_bc", todo!())
        .route("/get_peer_list", todo!())
        .route("/get_public_nodes", todo!())
        .route("/set_log_hash_rate", todo!())
        .route("/set_log_level", todo!())
        .route("/set_log_categories", todo!())
        .route("/get_transaction_pool", todo!())
        .route("/get_transaction_pool_hashes", todo!())
        .route("/get_transaction_pool_stats", todo!())
        .route("/set_bootstrap_daemon", todo!())
        .route("/stop_daemon", todo!())
        .route("/get_info", todo!())
        .route("/getinfo", todo!())
        .route("/get_net_stats", todo!())
        .route("/get_limit", todo!())
        .route("/set_limit", todo!())
        .route("/out_peers", todo!())
        .route("/in_peers", todo!())
        .route("/get_outs", todo!())
        .route("/update", todo!())
        .route("/pop_blocks", todo!())
        // Binary routes.
        .route("/get_blocks.bin", todo!())
        .route("/getblocks.bin", todo!())
        .route("/get_blocks_by_height.bin", todo!())
        .route("/getblocks_by_height.bin", todo!())
        .route("/get_hashes.bin", todo!())
        .route("/gethashes.bin", todo!())
        .route("/get_o_indexes.bin", todo!())
        .route("/get_outs.bin", todo!())
        .route("/get_transaction_pool_hashes.bin", todo!())
        .route("/get_output_distribution.bin", todo!())
        // Unknown route.
        .route("/*", todo!())
}
