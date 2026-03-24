//! Free functions.

//---------------------------------------------------------------------------------------------------- Use
use axum::Router;

use crate::{
    route::{bin, fallback, json_rpc, other_json},
    rpc_handler::RpcHandler,
};

//---------------------------------------------------------------------------------------------------- RouterBuilder
/// Generate the `RouterBuilder` struct.
macro_rules! generate_router_builder {
    ($(
        // Syntax:
        // $BUILDER_FUNCTION_NAME =>
        // $ACTUAL_ENDPOINT_STRING =>
        // $ENDPOINT_FUNCTION_MODULE::$ENDPOINT_FUNCTION =>
        // ($HTTP_METHOD(s))
        $endpoint_ident:ident =>
        $endpoint_string:literal =>
        $endpoint_module:ident::$endpoint_fn:ident =>
        ($($http_method:ident),*)
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
        /// - `bin_` (e.g. [`bin_get_blocks`](RouterBuilder::bin_get_blocks))
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
        ///     .bin_get_outs()
        ///     .fallback()
        ///     .build();
        ///
        /// // Create a router with all endpoints enabled.
        /// let all = RouterBuilder::<RpcHandlerDummy>::new()
        ///     .all()
        ///     .build();
        /// ```
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
                        router: self.router.route(
                            $endpoint_string,
                            ::axum::routing::method_routing::MethodRouter::new()
                                $(.$http_method($endpoint_module::$endpoint_fn::<H>))*
                        ),
                    }
                }
            )*
        }
    };
}

generate_router_builder! {
    // JSON-RPC 2.0 route.
    json_rpc => "/json_rpc" => json_rpc::json_rpc => (get, post),

    // Other JSON routes.
    other_get_height                  => "/get_height"                  => other_json::get_height                  => (get, post),
    other_getheight                   => "/getheight"                   => other_json::get_height                  => (get, post),
    other_get_transactions            => "/get_transactions"            => other_json::get_transactions            => (get, post),
    other_gettransactions             => "/gettransactions"             => other_json::get_transactions            => (get, post),
    other_get_alt_blocks_hashes       => "/get_alt_blocks_hashes"       => other_json::get_alt_blocks_hashes       => (get, post),
    other_is_key_image_spent          => "/is_key_image_spent"          => other_json::is_key_image_spent          => (get, post),
    other_send_raw_transaction        => "/send_raw_transaction"        => other_json::send_raw_transaction        => (get, post),
    other_sendrawtransaction          => "/sendrawtransaction"          => other_json::send_raw_transaction        => (get, post),
    other_start_mining                => "/start_mining"                => other_json::start_mining                => (get, post),
    other_stop_mining                 => "/stop_mining"                 => other_json::stop_mining                 => (get, post),
    other_mining_status               => "/mining_status"               => other_json::mining_status               => (get, post),
    other_save_bc                     => "/save_bc"                     => other_json::save_bc                     => (get, post),
    other_get_peer_list               => "/get_peer_list"               => other_json::get_peer_list               => (get, post),
    other_get_public_nodes            => "/get_public_nodes"            => other_json::get_public_nodes            => (get, post),
    other_set_log_hash_rate           => "/set_log_hash_rate"           => other_json::set_log_hash_rate           => (get, post),
    other_set_log_level               => "/set_log_level"               => other_json::set_log_level               => (get, post),
    other_set_log_categories          => "/set_log_categories"          => other_json::set_log_categories          => (get, post),
    other_get_transaction_pool        => "/get_transaction_pool"        => other_json::get_transaction_pool        => (get, post),
    other_get_transaction_pool_hashes => "/get_transaction_pool_hashes" => other_json::get_transaction_pool_hashes => (get, post),
    other_get_transaction_pool_stats  => "/get_transaction_pool_stats"  => other_json::get_transaction_pool_stats  => (get, post),
    other_set_bootstrap_daemon        => "/set_bootstrap_daemon"        => other_json::set_bootstrap_daemon        => (get, post),
    other_stop_daemon                 => "/stop_daemon"                 => other_json::stop_daemon                 => (get, post),
    other_get_net_stats               => "/get_net_stats"               => other_json::get_net_stats               => (get, post),
    other_get_limit                   => "/get_limit"                   => other_json::get_limit                   => (get, post),
    other_set_limit                   => "/set_limit"                   => other_json::set_limit                   => (get, post),
    other_out_peers                   => "/out_peers"                   => other_json::out_peers                   => (get, post),
    other_in_peers                    => "/in_peers"                    => other_json::in_peers                    => (get, post),
    other_get_outs                    => "/get_outs"                    => other_json::get_outs                    => (get, post),
    other_update                      => "/update"                      => other_json::update                      => (get, post),
    other_pop_blocks                  => "/pop_blocks"                  => other_json::pop_blocks                  => (get, post),

    // Binary routes.
    bin_get_blocks                  => "/get_blocks.bin"                  => bin::get_blocks                  => (get, post),
    bin_getblocks                   => "/getblocks.bin"                   => bin::get_blocks                  => (get, post),
    bin_get_blocks_by_height        => "/get_blocks_by_height.bin"        => bin::get_blocks_by_height        => (get, post),
    bin_getblocks_by_height         => "/getblocks_by_height.bin"         => bin::get_blocks_by_height        => (get, post),
    bin_get_hashes                  => "/get_hashes.bin"                  => bin::get_hashes                  => (get, post),
    bin_gethashes                   => "/gethashes.bin"                   => bin::get_hashes                  => (get, post),
    bin_get_o_indexes               => "/get_o_indexes.bin"               => bin::get_o_indexes               => (get, post),
    bin_get_outs                    => "/get_outs.bin"                    => bin::get_outs                    => (get, post),
    bin_get_transaction_pool_hashes => "/get_transaction_pool_hashes.bin" => other_json::get_transaction_pool_hashes => (get, post),
    bin_get_output_distribution     => "/get_output_distribution.bin"     => bin::get_output_distribution     => (get, post),
}

impl<H: RpcHandler> Default for RouterBuilder<H> {
    /// Uses [`Self::all`].
    fn default() -> Self {
        Self::new().all()
    }
}
