//! Free functions.

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, marker::PhantomData};

use axum::{extract::State, routing::method_routing::get, Router};
use tower::Service;

use crate::{
    error::Error, request::Request, response::Response, route, rpc_handler::RpcHandler, RpcState,
};

//---------------------------------------------------------------------------------------------------- Router
/// TODO
#[allow(clippy::needless_pass_by_value)]
pub fn create_router<H: RpcHandler>() -> Router<H> {
    // List of `monerod` routes:
    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L97-L189>

    let mut router = Router::new();

    // JSON-RPC route.
    router = router.route("/json_rpc", get(route::json_rpc::<H>));

    // Other JSON routes.
    for other_route in [
        "/get_height",
        "/getheight",
        "/get_transactions",
        "/gettransactions",
        "/get_alt_blocks_hashes",
        "/is_key_image_spent",
        "/send_raw_transaction",
        "/sendrawtransaction",
        "/start_mining",
        "/stop_mining",
        "/mining_status",
        "/save_bc",
        "/get_peer_list",
        "/get_public_nodes",
        "/set_log_hash_rate",
        "/set_log_level",
        "/set_log_categories",
        "/get_transaction_pool",
        "/get_transaction_pool_hashes",
        "/get_transaction_pool_stats",
        "/set_bootstrap_daemon",
        "/stop_daemon",
        "/get_info",
        "/getinfo",
        "/get_net_stats",
        "/get_limit",
        "/set_limit",
        "/out_peers",
        "/in_peers",
        "/get_outs",
        "/update",
        "/pop_blocks",
    ] {
        router = router.route(other_route, get(route::other::<H>));
    }

    // Binary routes.
    for binary_route in [
        "/get_blocks.bin",
        "/getblocks.bin",
        "/get_blocks_by_height.bin",
        "/getblocks_by_height.bin",
        "/get_hashes.bin",
        "/gethashes.bin",
        "/get_o_indexes.bin",
        "/get_outs.bin",
        "/get_transaction_pool_hashes.bin",
        "/get_output_distribution.bin",
    ] {
        router = router.route(binary_route, get(route::bin::<H>));
    }

    // Unknown route.
    router = router.route("/*", get(route::unknown));

    router
}
