//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.

//---------------------------------------------------------------------------------------------------- Import
use crate::rpc::types::macros::define_request_and_response;

//---------------------------------------------------------------------------------------------------- Struct definitions
// This generates 2 const strings:
//
// - `const GET_BLOCK_TEMPLATE_REQUEST: &str = "..."`
// - `const GET_BLOCK_TEMPLATE_RESPONSE: &str = "..."`
//
// with some interconnected documentation.
define_request_and_response! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_block_template,

    // The base const name: the type of the request/response.
    GET_BLOCK_TEMPLATE: &str,

    // The request literal.
    Request = "";

    // The response literal.
    Response = "";
}

// define_request_and_response! {
//     get_block_count,
//     GetBlockCount,
//     Request {},
//     Response {
//     }
// }

// define_request_and_response! {
//     on_get_block_hash,
//     OnGetBlockHash,
//     /// ```rust
//     /// use serde_json::*;
//     /// use cuprate_rpc_types::json::*;
//     ///
//     /// let x = OnGetBlockHashRequest { block_height: [3] };
//     /// let x = to_string(&x).unwrap();
//     /// assert_eq!(x, "[3]");
//     /// ```
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     #[derive(Copy)]
//     Request {
//         // This is `std::vector<u64>` in `monerod` but
//         // it must be a 1 length array or else it will error.
//         block_height: [u64; 1],
//     },
//     /// ```rust
//     /// use serde_json::*;
//     /// use cuprate_rpc_types::json::*;
//     ///
//     /// let x = OnGetBlockHashResponse { block_hash: String::from("asdf") };
//     /// let x = to_string(&x).unwrap();
//     /// assert_eq!(x, "\"asdf\"");
//     /// ```
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Response {
//         block_hash: String,
//     }
// }

// define_request_and_response! {
//     submit_block,
//     SubmitBlock,
//     /// ```rust
//     /// use serde_json::*;
//     /// use cuprate_rpc_types::json::*;
//     ///
//     /// let x = SubmitBlockRequest { block_blob: ["a".into()] };
//     /// let x = to_string(&x).unwrap();
//     /// assert_eq!(x, r#"["a"]"#);
//     /// ```
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Request {
//         // This is `std::vector<std::string>` in `monerod` but
//         // it must be a 1 length array or else it will error.
//         block_blob: [String; 1],
//     },
//     ResponseBase {
//         block_id: String,
//     }
// }

// define_request_and_response! {
//     generateblocks,
//     GenerateBlocks,
//     Request {
//         amount_of_blocks: u64,
//         prev_block: String,
//         starting_nonce: u32,
//         wallet_address: String,
//     },
//     ResponseBase {
//         blocks: Vec<String>,
//         height: u64,
//     }
// }

// define_request_and_response! {
//     get_last_block_header,
//     GetLastBlockHeader,
//     #[derive(Copy)]
//     Request {
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     AccessResponseBase {
//         block_header: BlockHeader,
//     }
// }

// define_request_and_response! {
//     get_block_header_by_hash,
//     GetBlockHeaderByHash,
//     Request {
//         hash: String,
//         hashes: Vec<String>,
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     AccessResponseBase {
//         block_header: BlockHeader,
//         block_headers: Vec<BlockHeader>,
//     }
// }

// define_request_and_response! {
//     get_block_header_by_height,
//     GetBlockHeaderByHeight,
//     #[derive(Copy)]
//     Request {
//         height: u64,
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     AccessResponseBase {
//         block_header: BlockHeader,
//     }
// }

// define_request_and_response! {
//     get_block_headers_range,
//     GetBlockHeadersRange,
//     #[derive(Copy)]
//     Request {
//         start_height: u64,
//         end_height: u64,
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     AccessResponseBase {
//         headers: Vec<BlockHeader>,
//     }
// }

// define_request_and_response! {
//     get_block,
//     GetBlock,
//     Request {
//         // `monerod` has both `hash` and `height` fields.
//         // In the RPC handler, if `hash.is_empty()`, it will use it, else, it uses `height`.
//         height: u64 = default_height(), "default_height",
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     AccessResponseBase {
//         blob: String,
//         block_header: BlockHeader,
//         json: String, // TODO: this should be defined in a struct, it has many fields.
//         miner_tx_hash: String,
//         tx_hashes: Vec<String>,
//     }
// }

// define_request_and_response! {
//     get_connections,
//     GetConnections,
//     Request {},
//     ResponseBase {
//         // FIXME: This is a `std::list` in `monerod` because...?
//         connections: Vec<ConnectionInfo>,
//     }
// }

// define_request_and_response! {
//     get_info,
//     GetInfo,
//     Request {},
//     AccessResponseBase {
//         adjusted_time: u64,
//         alt_blocks_count: u64,
//         block_size_limit: u64,
//         block_size_median: u64,
//         block_weight_limit: u64,
//         block_weight_median: u64,
//         bootstrap_daemon_address: String,
//         busy_syncing: bool,
//         cumulative_difficulty_top64: u64,
//         cumulative_difficulty: u64,
//         database_size: u64,
//         difficulty_top64: u64,
//         difficulty: u64,
//         free_space: u64,
//         grey_peerlist_size: u64,
//         height: u64,
//         height_without_bootstrap: u64,
//         incoming_connections_count: u64,
//         mainnet: bool,
//         nettype: String,
//         offline: bool,
//         outgoing_connections_count: u64,
//         restricted: bool,
//         rpc_connections_count: u64,
//         stagenet: bool,
//         start_time: u64,
//         synchronized: bool,
//         target_height: u64,
//         target: u64,
//         testnet: bool,
//         top_block_hash: String,
//         tx_count: u64,
//         tx_pool_size: u64,
//         update_available: bool,
//         version: String,
//         was_bootstrap_ever_used: bool,
//         white_peerlist_size: u64,
//         wide_cumulative_difficulty: String,
//         wide_difficulty: String,
//     }
// }

// define_request_and_response! {
//     hard_fork_info,
//     HardForkInfo,
//     Request {},
//     AccessResponseBase {
//         earliest_height: u64,
//         enabled: bool,
//         state: u32,
//         threshold: u32,
//         version: u8,
//         votes: u32,
//         voting: u8,
//         window: u32,
//     }
// }

// define_request_and_response! {
//     set_bans,
//     SetBans,
//     Request {
//         bans: Vec<SetBan>,
//     },
//     ResponseBase {}
// }

// define_request_and_response! {
//     get_bans,
//     GetBans,
//     Request {},
//     ResponseBase {
//         bans: Vec<GetBan>,
//     }
// }

// define_request_and_response! {
//     banned,
//     Banned,
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Request {
//         address: String,
//     },
//     #[derive(Copy)]
//     Response {
//         banned: bool,
//         seconds: u32,
//         status: Status,
//     }
// }

// define_request_and_response! {
//     flush_txpool,
//     FlushTransactionPool,
//     Request {
//         txids: Vec<String> = default_vec::<String>(), "default_vec",
//     },
//     #[derive(Copy)]
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Response {
//         status: Status,
//     }
// }

// define_request_and_response! {
//     get_output_histogram,
//     GetOutputHistogram,
//     Request {
//         amounts: Vec<u64>,
//         min_count: u64,
//         max_count: u64,
//         unlocked: bool,
//         recent_cutoff: u64,
//     },
//     AccessResponseBase {
//         histogram: Vec<HistogramEntry>,
//     }
// }

// define_request_and_response! {
//     get_coinbase_tx_sum,
//     GetCoinbaseTxSum,
//     Request {
//         height: u64,
//         count: u64,
//     },
//     AccessResponseBase {
//         emission_amount: u64,
//         emission_amount_top64: u64,
//         fee_amount: u64,
//         fee_amount_top64: u64,
//         wide_emission_amount: String,
//         wide_fee_amount: String,
//     }
// }

// define_request_and_response! {
//     get_version,
//     GetVersion,
//     Request {},
//     ResponseBase {
//         version: u32,
//         release: bool,
//         #[serde(skip_serializing_if = "is_zero")]
//         current_height: u64 = default_zero(), "default_zero",
//         #[serde(skip_serializing_if = "is_zero")]
//         target_height: u64 = default_zero(), "default_zero",
//         #[serde(skip_serializing_if = "Vec::is_empty")]
//         hard_forks: Vec<HardforkEntry> = default_vec(), "default_vec",
//     }
// }

// define_request_and_response! {
//     get_fee_estimate,
//     GetFeeEstimate,
//     Request {},
//     AccessResponseBase {
//         fee: u64,
//         fees: Vec<u64>,
//         #[serde(skip_serializing_if = "is_one")]
//         quantization_mask: u64,
//     }
// }

// define_request_and_response! {
//     get_alternate_chains,
//     GetAlternateChains,
//     Request {},
//     ResponseBase {
//         chains: Vec<ChainInfo>,
//     }
// }

// define_request_and_response! {
//     relay_tx,
//     RelayTx,
//     Request {
//         txids: Vec<String>,
//     },
//     #[derive(Copy)]
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Response {
//         status: Status,
//     }
// }

// define_request_and_response! {
//     sync_info,
//     SyncInfo,
//     Request {},
//     AccessResponseBase {
//         height: u64,
//         next_needed_pruning_seed: u32,
//         overview: String,
//         // FIXME: This is a `std::list` in `monerod` because...?
//         peers: Vec<SyncInfoPeer>,
//         // FIXME: This is a `std::list` in `monerod` because...?
//         spans: Vec<Span>,
//         target_height: u64,
//     }
// }

// define_request_and_response! {
//     get_txpool_backlog,
//     GetTransactionPoolBacklog,
//     Request {},
//     ResponseBase {
//         // TODO: this is a [`BinaryString`].
//         backlog: Vec<TxBacklogEntry>,
//     }
// }

// define_request_and_response! {
//     get_output_distribution,
//     /// This type is also used in the (undocumented)
//     GetOutputDistribution,
//     Request {
//         amounts: Vec<u64>,
//         binary: bool,
//         compress: bool,
//         cumulative: bool,
//         from_height: u64,
//         to_height: u64,
//     },
//     /// TODO: this request has custom serde:
//         distributions: Vec<OutputDistributionData>,
//     }
// }

// define_request_and_response! {
//     get_miner_data,
//     GetMinerData,
//     Request {},
//     ResponseBase {
//         major_version: u8,
//         height: u64,
//         prev_id: String,
//         seed_hash: String,
//         difficulty: String,
//         median_weight: u64,
//         already_generated_coins: u64,
//     }
// }

// define_request_and_response! {
//     prune_blockchain,
//     PruneBlockchain,
//     #[derive(Copy)]
//     Request {
//         check: bool = default_false(), "default_false",
//     },
//     #[derive(Copy)]
//     ResponseBase {
//         pruned: bool,
//         pruning_seed: u32,
//     }
// }

// define_request_and_response! {
//     calc_pow,
//     CalcPow,
//     Request {
//         major_version: u8,
//         height: u64,
//         block_blob: String,
//         seed_hash: String,
//     },
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Response {
//         pow_hash: String,
//     }
// }

// define_request_and_response! {
//     flush_cache,
//     FlushCache,
//     #[derive(Copy)]
//     Request {
//         bad_txs: bool = default_false(), "default_false",
//         bad_blocks: bool = default_false(), "default_false",
//     },
//     ResponseBase {}
// }

// define_request_and_response! {
//     add_aux_pow,
//     AddAuxPow,
//     Request {
//         blocktemplate_blob: String,
//         aux_pow: Vec<AuxPow>,
//     },
//     ResponseBase {
//       blocktemplate_blob: String,
//       blockhashing_blob: String,
//       merkle_root: String,
//       merkle_tree_depth: u64,
//       aux_pow: Vec<AuxPow>,
//     }
// }

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
