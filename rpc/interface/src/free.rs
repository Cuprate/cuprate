//! Free functions.

//---------------------------------------------------------------------------------------------------- Use
use axum::{routing::method_routing::get, Router};

use crate::{
    route::{bin, fallback, json, other},
    rpc_handler::RpcHandler,
};

//---------------------------------------------------------------------------------------------------- Router
/// Create the RPC [`axum::Router`].
///
/// This creates an [`axum::Router`] with all Monero RPC routes
/// registered and handled by your [`RpcHandler`] of choice.
///
/// # Routes
/// For a list of all `monerod` routes, [here](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L97-L189), or the source code of this function.
#[rustfmt::skip]
pub fn create_router<H: RpcHandler>() -> Router<H> {
    Router::new()
        // JSON-RPC route.
        .route("/json_rpc", get(json::json_rpc::<H>))
        // Other JSON routes.
        .route("/get_height", get(other::get_height::<H>))
        .route("/getheight", get(other::get_height::<H>))
        .route("/get_transactions", get(other::get_transactions::<H>))
        .route("/gettransactions", get(other::get_transactions::<H>))
        .route("/get_alt_blocks_hashes", get(other::get_alt_blocks_hashes::<H>))
        .route("/is_key_image_spent", get(other::is_key_image_spent::<H>))
        .route("/send_raw_transaction", get(other::send_raw_transaction::<H>))
        .route("/sendrawtransaction", get(other::send_raw_transaction::<H>))
        .route("/start_mining", get(other::start_mining::<H>))
        .route("/stop_mining", get(other::stop_mining::<H>))
        .route("/mining_status", get(other::mining_status::<H>))
        .route("/save_bc", get(other::save_bc::<H>))
        .route("/get_peer_list", get(other::get_peer_list::<H>))
        .route("/get_public_nodes", get(other::get_public_nodes::<H>))
        .route("/set_log_hash_rate", get(other::set_log_hash_rate::<H>))
        .route("/set_log_level", get(other::set_log_level::<H>))
        .route("/set_log_categories", get(other::set_log_categories::<H>))
        .route("/get_transaction_pool", get(other::get_transaction_pool::<H>))
        .route("/get_transaction_pool_hashes", get(other::get_transaction_pool_hashes::<H>))
        .route("/get_transaction_pool_stats", get(other::get_transaction_pool_stats::<H>))
        .route("/set_bootstrap_daemon", get(other::set_bootstrap_daemon::<H>))
        .route("/stop_daemon", get(other::stop_daemon::<H>))
        .route("/get_net_stats", get(other::get_net_stats::<H>))
        .route("/get_limit", get(other::get_limit::<H>))
        .route("/set_limit", get(other::set_limit::<H>))
        .route("/out_peers", get(other::out_peers::<H>))
        .route("/in_peers", get(other::in_peers::<H>))
        .route("/get_outs", get(other::get_outs::<H>))
        .route("/update", get(other::update::<H>))
        .route("/pop_blocks", get(other::pop_blocks::<H>))
        // Binary routes.
        .route("/get_blocks.bin", get(bin::get_blocks::<H>))
        .route("/getblocks.bin", get(bin::get_blocks::<H>))
        .route("/get_blocks_by_height.bin", get(bin::get_blocks_by_height::<H>))
        .route("/getblocks_by_height.bin", get(bin::get_blocks_by_height::<H>))
        .route("/get_hashes.bin", get(bin::get_hashes::<H>))
        .route("/gethashes.bin", get(bin::get_hashes::<H>))
        .route("/get_o_indexes.bin", get(bin::get_o_indexes::<H>))
        .route("/get_outs.bin", get(bin::get_outs::<H>))
        .route("/get_transaction_pool_hashes.bin", get(bin::get_transaction_pool_hashes::<H>))
        .route("/get_output_distribution.bin", get(bin::get_output_distribution::<H>))
        // Fallback route (catch-all).
        //
        // Deprecated routes will also route here, list:
        // - `get_info`
        // - `getinfo`
        .fallback(fallback::fallback)
}
