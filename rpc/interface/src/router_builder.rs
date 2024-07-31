//! Free functions.

use std::marker::PhantomData;

//---------------------------------------------------------------------------------------------------- Use
use axum::{routing::method_routing::get, Router};

use crate::{
    route::{bin, fallback, json, other},
    rpc_handler::RpcHandler,
};

//---------------------------------------------------------------------------------------------------- RouterBuilder
/// Generate the `RouterBuilder` struct.
macro_rules! generate_router_builder {
    ($(
        // Syntax:
        // $BUILDER_FUNCTION_NAME => $ACTUAL_ENDPOINT => $ENDPOINT_FUNCTION
        $endpoint_ident:ident => $endpoint_string:literal => $endpoint_fn:expr
    ),* $(,)?) => {
        /// Builder for creating the RPC router.
        ///
        /// This builder allows you to selectively enable endpoints for the router,
        /// and a [`fallback`](RouterBuilder::fallback) route.
        ///
        /// The [`default`](RouterBuilder::default) is to enable [`all`](RouterBuilder::all) routes.
        ///
        /// # Routes
        /// Functions that enable routes are separated into 3 groups:
        /// - `json_rpc` (enables all of JSON RPC 2.0)
        /// - `other_` (e.g. [`other_get_height`](RouterBuilder::other_get_height))
        /// - `binary_` (e.g. [`binary_get_blocks`](RouterBuilder::binary_get_blocks))
        ///
        /// For a list of all `monerod` routes, see
        /// [here](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L97-L189),
        /// or the source file of this type.
        ///
        /// # Aliases
        /// Some routes have aliases, such as [`/get_height`](RouterBuilder::other_get_height)
        /// and [`/getheight`](RouterBuilder::other_getheight).
        ///
        /// These both route to the same handler function, but they do not enable each other.
        ///
        /// If desired, you can enable `/get_height` but not `/getheight`.
        ///
        /// # Example
        /// ```rust
        /// use cuprate_rpc_interface::{RouterBuilder, RpcHandlerDummy};
        ///
        /// // Create a router with _only_ `/json_rpc` enabled.
        /// let only_json_rpc = RouterBuilder::<RpcHandlerDummy>::new()
        ///     .json_rpc()
        ///     .build();
        ///
        /// // Create a router with:
        /// // - `/get_outs.bin` enabled
        /// // - A fallback enabled
        /// let get_outs_bin_and_fallback = RouterBuilder::<RpcHandlerDummy>::new()
        ///     .binary_get_outs()
        ///     .fallback()
        ///     .build();
        ///
        /// // Create a router with all endpoints enabled.
        /// let all = RouterBuilder::<RpcHandlerDummy>::new()
        ///     .all()
        ///     .build();
        /// ```
        #[allow(clippy::struct_excessive_bools)]
        #[derive(Clone)]
        pub struct RouterBuilder<H: RpcHandler> {
            router: Router<H>,
        }

        impl<H: RpcHandler> RouterBuilder<H> {
            /// Create a new [`Self`].
            #[must_use]
            pub fn new() -> Self {
                Self {
                    router: Router::new(),
                }
            }

            /// Build [`Self`] into a [`Router`].
            ///
            /// All endpoints enabled in [`RouterBuilder`]
            /// will be enabled in this [`Router`].
            pub fn build(self) -> Router<H> {
                self.router
            }

            /// Enable all endpoints, including [`Self::fallback`].
            #[must_use]
            pub fn all(mut self) -> Self {
                $(
                    self = self.$endpoint_ident();
                )*

                self.fallback()
            }

            /// Enable the catch-all fallback route.
            ///
            /// Any unknown or disabled route will route here, e.g.:
            /// - `get_info`
            /// - `getinfo`
            /// - `asdf`
            #[must_use]
            pub fn fallback(self) -> Self {
                Self {
                    router: self.router.fallback(fallback::fallback),
                }
            }

            $(
                #[doc = concat!(
                    "Enable the `",
                    $endpoint_string,
                    "` endpoint.",
                )]
                #[must_use]
                pub fn $endpoint_ident(self) -> Self {
                    Self {
                        router: self.router.route($endpoint_string, $endpoint_fn),
                    }
                }
            )*
        }
    };
}

generate_router_builder! {
    // JSON-RPC 2.0 route.
    json_rpc => "/json_rpc" => get(json::json_rpc::<H>),
    // Other JSON routes.
    other_get_height => "/get_height" => get(other::get_height::<H>),
    other_getheight => "/getheight" => get(other::get_height::<H>),
    other_get_transactions => "/get_transactions" => get(other::get_transactions::<H>),
    other_gettransactions => "/gettransactions" => get(other::get_transactions::<H>),
    other_get_alt_blocks_hashes => "/get_alt_blocks_hashes" => get(other::get_alt_blocks_hashes::<H>),
    other_is_key_image_spent => "/is_key_image_spent" => get(other::is_key_image_spent::<H>),
    other_send_raw_transaction => "/send_raw_transaction" => get(other::send_raw_transaction::<H>),
    other_sendrawtransaction => "/sendrawtransaction" => get(other::send_raw_transaction::<H>),
    other_start_mining => "/start_mining" => get(other::start_mining::<H>),
    other_stop_mining => "/stop_mining" => get(other::stop_mining::<H>),
    other_mining_status => "/mining_status" => get(other::mining_status::<H>),
    other_save_bc => "/save_bc" => get(other::save_bc::<H>),
    other_get_peer_list => "/get_peer_list" => get(other::get_peer_list::<H>),
    other_get_public_nodes => "/get_public_nodes" => get(other::get_public_nodes::<H>),
    other_set_log_hash_rate => "/set_log_hash_rate" => get(other::set_log_hash_rate::<H>),
    other_set_log_level => "/set_log_level" => get(other::set_log_level::<H>),
    other_set_log_categories => "/set_log_categories" => get(other::set_log_categories::<H>),
    other_get_transaction_pool => "/get_transaction_pool" => get(other::get_transaction_pool::<H>),
    other_get_transaction_pool_hashes => "/get_transaction_pool_hashes" => get(other::get_transaction_pool_hashes::<H>),
    other_get_transaction_pool_stats => "/get_transaction_pool_stats" => get(other::get_transaction_pool_stats::<H>),
    other_set_bootstrap_daemon => "/set_bootstrap_daemon" => get(other::set_bootstrap_daemon::<H>),
    other_stop_daemon => "/stop_daemon" => get(other::stop_daemon::<H>),
    other_get_net_stats => "/get_net_stats" => get(other::get_net_stats::<H>),
    other_get_limit => "/get_limit" => get(other::get_limit::<H>),
    other_set_limit => "/set_limit" => get(other::set_limit::<H>),
    other_out_peers => "/out_peers" => get(other::out_peers::<H>),
    other_in_peers => "/in_peers" => get(other::in_peers::<H>),
    other_get_outs => "/get_outs" => get(other::get_outs::<H>),
    other_update => "/update" => get(other::update::<H>),
    other_pop_blocks => "/pop_blocks" => get(other::pop_blocks::<H>),
    // Binary routes.
    binary_get_blocks => "/get_blocks.bin" => get(bin::get_blocks::<H>),
    binary_getblocks => "/getblocks.bin" => get(bin::get_blocks::<H>),
    binary_get_blocks_by_height => "/get_blocks_by_height.bin" => get(bin::get_blocks_by_height::<H>),
    binary_getblocks_by_height => "/getblocks_by_height.bin" => get(bin::get_blocks_by_height::<H>),
    binary_get_hashes => "/get_hashes.bin" => get(bin::get_hashes::<H>),
    binary_gethashes => "/gethashes.bin" => get(bin::get_hashes::<H>),
    binary_get_o_indexes => "/get_o_indexes.bin" => get(bin::get_o_indexes::<H>),
    binary_get_outs => "/get_outs.bin" => get(bin::get_outs::<H>),
    binary_get_transaction_pool_hashes => "/get_transaction_pool_hashes.bin" => get(bin::get_transaction_pool_hashes::<H>),
    binary_get_output_distribution => "/get_output_distribution.bin" => get(bin::get_output_distribution::<H>),
}

impl<H: RpcHandler> Default for RouterBuilder<H> {
    /// Uses [`Self::all`].
    fn default() -> Self {
        Self::new().all()
    }
}
