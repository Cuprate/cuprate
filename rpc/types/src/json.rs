//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{default_false, default_height, default_string, default_vec, default_zero},
    free::{is_one, is_zero},
    macros::{define_request_and_response, json_rpc_doc_test},
    misc::{
        AuxPow, BlockHeader, ChainInfo, ConnectionInfo, Distribution, GetBan, HardforkEntry,
        HistogramEntry, OutputDistributionData, SetBan, Span, Status, SyncInfoPeer, TxBacklogEntry,
    },
};

//---------------------------------------------------------------------------------------------------- Definitions

// This generates 2 structs:
//
// - `GetBlockTemplateRequest`
// - `GetBlockTemplateResponse`
//
// with some interconnected documentation.
define_request_and_response! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_block_template,

    // The commit hash and `$file.$extension` in which this type is defined in
    // the Monero codebase in the `rpc/` directory, followed by the specific lines.
    cc73fe71162d564ffda8e549b79a350bca53c454 => core_rpc_server_commands_defs.h => 943..=994,

    // The base type name.
    GetBlockTemplate,

    // The request type.
    //
    // If `Request {/* fields */}` is provided, a struct is generate as-is.
    //
    // If `Request {}` is specified here, it will create a `pub type YOUR_REQUEST_TYPE = ()`
    // instead of a `struct`, see below in other macro definitions for an example.
    //
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    //
    #[doc = json_rpc_doc_test!(
        GET_BLOCK_TEMPLATE_REQUEST => GetBlockTemplateRequest {
            extra_nonce: String::default(),
            prev_block: String::default(),
            reserve_size: 60,
            wallet_address: "44GBHzv6ZyQdJkjqZje6KLZ3xSyN1hBSFAnLP6EAqJtCRVzMzZmeXTC2AHKDS9aEDTRKmo6a6o9r9j86pYfhCWDkKjbtcns".into(),
        }
    )]
    Request {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.

        // This optional expression can be placed after
        // a `field: field_type`. this indicates to the
        // macro to (de)serialize this field using this
        // default expression if it doesn't exist in epee.
        //
        // See `cuprate_epee_encoding::epee_object` for info.
        //
        // The default function must be specified twice:
        //
        // 1. As an expression
        // 2. As a string literal
        //
        // For example: `extra_nonce: String /* = default_string(), "default_string" */,`
        //
        // This is a HACK since `serde`'s default attribute only takes in
        // string literals and macros (stringify) within attributes do not work.
        extra_nonce: String = default_string(), "default_string",
        prev_block: String = default_string(), "default_string",

        // Another optional expression:
        // This indicates to the macro to (de)serialize
        // this field as another type in epee.
        //
        // See `cuprate_epee_encoding::epee_object` for info.
        reserve_size: u64 /* as Type */,

        wallet_address: String,
    },

    // The response type.
    //
    // If `Response {/* fields */}` is used,
    // this will generate a struct as-is.
    //
    // If a type found in [`crate::base`] is used,
    // It acts as a "base" that gets flattened into
    // the actual request type.
    //
    // "Flatten" means the field(s) of a struct gets inlined
    // directly into the struct during (de)serialization, see:
    // <https://serde.rs/field-attrs.html#flatten>.
    #[doc = json_rpc_doc_test!(
        GET_BLOCK_TEMPLATE_RESPONSE => GetBlockTemplateResponse {
            base: ResponseBase::ok(),
            blockhashing_blob: "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a00000000e0c20372be23d356347091025c5b5e8f2abf83ab618378565cce2b703491523401".into(),
            blocktemplate_blob: "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
            difficulty_top64: 0,
            difficulty: 283305047039,
            expected_reward: 600000000000,
            height: 3195018,
            next_seed_hash: "".into(),
            prev_hash: "9d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a".into(),
            reserved_offset: 131,
            seed_hash: "e2aa0b7b55042cd48b02e395d78fa66a29815ccc1584e38db2d1f0e8485cd44f".into(),
            seed_height: 3194880,
            wide_difficulty: "0x41f64bf3ff".into(),
        }
    )]
    ResponseBase {
        // This is using [`crate::base::ResponseBase`],
        // so the type we generate will contain this field:
        // ```
        // base: crate::base::ResponseBase,
        // ```
        //
        // This is flattened with serde and epee, so during
        // (de)serialization, it will act as if there are 2 extra fields here:
        // ```
        // status: crate::Status,
        // untrusted: bool,
        // ```
        blockhashing_blob: String,
        blocktemplate_blob: String,
        difficulty_top64: u64,
        difficulty: u64,
        expected_reward: u64,
        height: u64,
        next_seed_hash: String,
        prev_hash: String,
        reserved_offset: u64,
        seed_hash: String,
        seed_height: u64,
        wide_difficulty: String,
    }
}

define_request_and_response! {
    get_block_count,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 919..=933,
    GetBlockCount,

    // There are no request fields specified,
    // this will cause the macro to generate a
    // type alias to `()` instead of a `struct`.
    Request {},

    #[doc = json_rpc_doc_test!(
        GET_BLOCK_COUNT_RESPONSE => GetBlockCountResponse {
            base: ResponseBase::ok(),
            count: 3195019,
        }
    )]
    ResponseBase {
        count: u64,
    }
}

define_request_and_response! {
    on_get_block_hash,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 935..=939,

    OnGetBlockHash,

    #[doc = json_rpc_doc_test!(
        ON_GET_BLOCK_HASH_REQUEST => OnGetBlockHashRequest {
            block_height: [912345],
        }
    )]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    #[derive(Copy)]
    Request {
        // This is `std::vector<u64>` in `monerod` but
        // it must be a 1 length array or else it will error.
        block_height: [u64; 1],
    },

    #[doc = json_rpc_doc_test!(
        ON_GET_BLOCK_HASH_RESPONSE => OnGetBlockHashResponse {
            block_hash: "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6".into(),
        }
    )]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        block_hash: String,
    }
}

define_request_and_response! {
    submit_block,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1114..=1128,

    SubmitBlock,

    #[doc = json_rpc_doc_test!(
        SUBMIT_BLOCK_REQUEST => SubmitBlockRequest {
            block_blob: ["0707e6bdfedc053771512f1bc27c62731ae9e8f2443db64ce742f4e57f5cf8d393de28551e441a0000000002fb830a01ffbf830a018cfe88bee283060274c0aae2ef5730e680308d9c00b6da59187ad0352efe3c71d36eeeb28782f29f2501bd56b952c3ddc3e350c2631d3a5086cac172c56893831228b17de296ff4669de020200000000".into()],
        }
    )]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Request {
        // This is `std::vector<std::string>` in `monerod` but
        // it must be a 1 length array or else it will error.
        block_blob: [String; 1],
    },

    // FIXME: `cuprate_test_utils` only has an `error` response for this.
    ResponseBase {
        block_id: String,
    }
}

define_request_and_response! {
    generateblocks,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1130..=1161,

    GenerateBlocks,

    #[doc = json_rpc_doc_test!(
        GENERATE_BLOCKS_REQUEST => GenerateBlocksRequest {
            amount_of_blocks: 1,
            prev_block: String::default(),
            wallet_address: "44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A".into(),
            starting_nonce: 0
        }
    )]
    Request {
        amount_of_blocks: u64,
        prev_block: String = default_string(), "default_string",
        starting_nonce: u32,
        wallet_address: String,
    },

    #[doc = json_rpc_doc_test!(
        GENERATE_BLOCKS_RESPONSE => GenerateBlocksResponse {
            base: ResponseBase::ok(),
            blocks: vec!["49b712db7760e3728586f8434ee8bc8d7b3d410dac6bb6e98bf5845c83b917e4".into()],
            height: 9783,
        }
    )]
    ResponseBase {
        blocks: Vec<String>,
        height: u64,
    }
}

define_request_and_response! {
    get_last_block_header,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1214..=1238,

    GetLastBlockHeader,

    #[derive(Copy)]
    Request {
        fill_pow_hash: bool = default_false(), "default_false",
    },

    #[doc = json_rpc_doc_test!(
        GET_LAST_BLOCK_HEADER_RESPONSE => GetLastBlockHeaderResponse {
            base: AccessResponseBase::ok(),
            block_header: BlockHeader {
                block_size: 200419,
                block_weight: 200419,
                cumulative_difficulty: 366125734645190820,
                cumulative_difficulty_top64: 0,
                depth: 0,
                difficulty: 282052561854,
                difficulty_top64: 0,
                hash: "57238217820195ac4c08637a144a885491da167899cf1d20e8e7ce0ae0a3434e".into(),
                height: 3195020,
                long_term_weight: 200419,
                major_version: 16,
                miner_tx_hash: "7a42667237d4f79891bb407c49c712a9299fb87fce799833a7b633a3a9377dbd".into(),
                minor_version: 16,
                nonce: 1885649739,
                num_txes: 37,
                orphan_status: false,
                pow_hash: "".into(),
                prev_hash: "22c72248ae9c5a2863c94735d710a3525c499f70707d1c2f395169bc5c8a0da3".into(),
                reward: 615702960000,
                timestamp: 1721245548,
                wide_cumulative_difficulty: "0x514bd6a74a7d0a4".into(),
                wide_difficulty: "0x41aba48bbe".into()
            }
        }
    )]
    AccessResponseBase {
        block_header: BlockHeader,
    }
}

define_request_and_response! {
    get_block_header_by_hash,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1240..=1269,
    GetBlockHeaderByHash,
    #[doc = json_rpc_doc_test!(
        GET_BLOCK_HEADER_BY_HASH_REQUEST => GetBlockHeaderByHashRequest {
            hash: "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6".into(),
            hashes: vec![],
            fill_pow_hash: false,
        }
    )]
    Request {
        hash: String,
        hashes: Vec<String> = default_vec::<String>(), "default_vec",
        fill_pow_hash: bool = default_false(), "default_false",
    },

    #[doc = json_rpc_doc_test!(
        GET_BLOCK_HEADER_BY_HASH_RESPONSE => GetBlockHeaderByHashResponse {
            base: AccessResponseBase::ok(),
            block_headers: vec![],
            block_header: BlockHeader {
                block_size: 210,
                block_weight: 210,
                cumulative_difficulty: 754734824984346,
                cumulative_difficulty_top64: 0,
                depth: 2282676,
                difficulty: 815625611,
                difficulty_top64: 0,
                hash: "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6".into(),
                height: 912345,
                long_term_weight: 210,
                major_version: 1,
                miner_tx_hash: "c7da3965f25c19b8eb7dd8db48dcd4e7c885e2491db77e289f0609bf8e08ec30".into(),
                minor_version: 2,
                nonce: 1646,
                num_txes: 0,
                orphan_status: false,
                pow_hash: "".into(),
                prev_hash: "b61c58b2e0be53fad5ef9d9731a55e8a81d972b8d90ed07c04fd37ca6403ff78".into(),
                reward: 7388968946286,
                timestamp: 1452793716,
                wide_cumulative_difficulty: "0x2ae6d65248f1a".into(),
                wide_difficulty: "0x309d758b".into()
            },
        }
    )]
    AccessResponseBase {
        block_header: BlockHeader,
        block_headers: Vec<BlockHeader> = default_vec::<BlockHeader>(), "default_vec",
    }
}

define_request_and_response! {
    get_block_header_by_height,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1271..=1296,

    GetBlockHeaderByHeight,

    #[derive(Copy)]
    #[doc = json_rpc_doc_test!(
        GET_BLOCK_HEADER_BY_HEIGHT_REQUEST => GetBlockHeaderByHeightRequest {
            height: 912345,
            fill_pow_hash: false,
        }
    )]
    Request {
        height: u64,
        fill_pow_hash: bool = default_false(), "default_false",
    },

    #[doc = json_rpc_doc_test!(
        GET_BLOCK_HEADER_BY_HEIGHT_RESPONSE => GetBlockHeaderByHeightResponse {
            base: AccessResponseBase::ok(),
            block_header: BlockHeader {
                block_size: 210,
                block_weight: 210,
                cumulative_difficulty: 754734824984346,
                cumulative_difficulty_top64: 0,
                depth: 2282677,
                difficulty: 815625611,
                difficulty_top64: 0,
                hash: "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6".into(),
                height: 912345,
                long_term_weight: 210,
                major_version: 1,
                miner_tx_hash: "c7da3965f25c19b8eb7dd8db48dcd4e7c885e2491db77e289f0609bf8e08ec30".into(),
                minor_version: 2,
                nonce: 1646,
                num_txes: 0,
                orphan_status: false,
                pow_hash: "".into(),
                prev_hash: "b61c58b2e0be53fad5ef9d9731a55e8a81d972b8d90ed07c04fd37ca6403ff78".into(),
                reward: 7388968946286,
                timestamp: 1452793716,
                wide_cumulative_difficulty: "0x2ae6d65248f1a".into(),
                wide_difficulty: "0x309d758b".into()
            },
        }
    )]
    AccessResponseBase {
        block_header: BlockHeader,
    }
}

// define_request_and_response! {
//     get_block_headers_range,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1756..=1783,
//     GetBlockHeadersRange,
//     #[derive(Copy)]
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         start_height: u64,
//         end_height: u64,
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     AccessResponseBase {
//         headers: Vec<BlockHeader>,
//     }
// }

// define_request_and_response! {
//     get_block,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1298..=1313,
//     GetBlock,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         // `monerod` has both `hash` and `height` fields.
//         // In the RPC handler, if `hash.is_empty()`, it will use it, else, it uses `height`.
//         // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2674>
//         hash: String = default_string(), "default_string",
//         height: u64 = default_height(), "default_height",
//         fill_pow_hash: bool = default_false(), "default_false",
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     AccessResponseBase {
//         blob: String,
//         block_header: BlockHeader,
//         json: String, // FIXME: this should be defined in a struct, it has many fields.
//         miner_tx_hash: String,
//         tx_hashes: Vec<String>,
//     }
// }

// define_request_and_response! {
//     get_connections,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1734..=1754,
//     GetConnections,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {
//         // FIXME: This is a `std::list` in `monerod` because...?
//         connections: Vec<ConnectionInfo>,
//     }
// }

// define_request_and_response! {
//     get_info,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 693..=789,
//     GetInfo,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
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
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1958..=1995,
//     HardForkInfo,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
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
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2032..=2067,
//     SetBans,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         bans: Vec<SetBan>,
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {}
// }

// define_request_and_response! {
//     get_bans,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1997..=2030,
//     GetBans,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {
//         bans: Vec<GetBan>,
//     }
// }

// define_request_and_response! {
//     banned,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2069..=2094,
//     Banned,
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         address: String,
//     },
//     Response {
//         banned: bool,
//         seconds: u32,
//         status: Status,
//     }
// }

// define_request_and_response! {
//     flush_txpool,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2096..=2116,
//     FlushTransactionPool,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         txids: Vec<String> = default_vec::<String>(), "default_vec",
//     },
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Response {
//         status: Status,
//     }
// }

// define_request_and_response! {
//     get_output_histogram,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2118..=2168,
//     GetOutputHistogram,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         amounts: Vec<u64>,
//         min_count: u64,
//         max_count: u64,
//         unlocked: bool,
//         recent_cutoff: u64,
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     AccessResponseBase {
//         histogram: Vec<HistogramEntry>,
//     }
// }

// define_request_and_response! {
//     get_coinbase_tx_sum,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2213..=2248,
//     GetCoinbaseTxSum,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         height: u64,
//         count: u64,
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
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
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2170..=2211,
//     GetVersion,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {
//         version: u32,
//         release: bool,
//         #[serde(skip_serializing_if = "is_zero")]
//         current_height: u64 = default_zero::<u64>(), "default_zero",
//         #[serde(skip_serializing_if = "is_zero")]
//         target_height: u64 = default_zero::<u64>(), "default_zero",
//         #[serde(skip_serializing_if = "Vec::is_empty")]
//         hard_forks: Vec<HardforkEntry> = default_vec(), "default_vec",
//     }
// }

// define_request_and_response! {
//     get_fee_estimate,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2250..=2277,
//     GetFeeEstimate,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     AccessResponseBase {
//         fee: u64,
//         fees: Vec<u64>,
//         #[serde(skip_serializing_if = "is_one")]
//         quantization_mask: u64,
//     }
// }

// define_request_and_response! {
//     get_alternate_chains,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2279..=2310,
//     GetAlternateChains,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {
//         chains: Vec<ChainInfo>,
//     }
// }

// define_request_and_response! {
//     relay_tx,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2361..=2381,
//     RelayTx,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         txids: Vec<String>,
//     },
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Response {
//         status: Status,
//     }
// }

// define_request_and_response! {
//     sync_info,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2383..=2443,
//     SyncInfo,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
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
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1637..=1664,
//     GetTransactionPoolBacklog,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {
//         // TODO: this is a [`BinaryString`].
//         backlog: Vec<TxBacklogEntry>,
//     }
// }

// define_request_and_response! {
//     get_output_distribution,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2445..=2520,
//     /// This type is also used in the (undocumented)
//     /// [`/get_output_distribution.bin`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L138)
//     /// binary endpoint.
//     GetOutputDistribution,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         amounts: Vec<u64>,
//         binary: bool,
//         compress: bool,
//         cumulative: bool,
//         from_height: u64,
//         to_height: u64,
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     AccessResponseBase {
//         distributions: Vec<Distribution>,
//     }
// }

// define_request_and_response! {
//     get_miner_data,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 996..=1044,
//     GetMinerData,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {},
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
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
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2747..=2772,
//     PruneBlockchain,
//     #[derive(Copy)]
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         check: bool = default_false(), "default_false",
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {
//         pruned: bool,
//         pruning_seed: u32,
//     }
// }

// define_request_and_response! {
//     calc_pow,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1046..=1066,
//     CalcPow,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
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
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 2774..=2796,
//     FlushCache,
//     #[derive(Copy)]
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         bad_txs: bool = default_false(), "default_false",
//         bad_blocks: bool = default_false(), "default_false",
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
//     ResponseBase {}
// }

// define_request_and_response! {
//     add_aux_pow,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1068..=1112,
//     AddAuxPow,
//     #[doc = json_rpc_doc_test!(
//         _REQUEST => Request {
//         }
//     )]
//     Request {
//         blocktemplate_blob: String,
//         aux_pow: Vec<AuxPow>,
//     },
//     #[doc = json_rpc_doc_test!(
//         _RESPONSE => Response {
//         }
//     )]
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
