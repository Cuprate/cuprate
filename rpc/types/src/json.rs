//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{
        default_false, default_height, default_one, default_string, default_true, default_vec,
        default_zero,
    },
    macros::define_request_and_response,
    misc::{
        AuxPow, BlockHeader, ChainInfo, ConnectionInfo, Distribution, GetBan,
        GetMinerDataTxBacklogEntry, HardforkEntry, HistogramEntry, SetBan, Span, Status,
        SyncInfoPeer, TxBacklogEntry,
    },
    rpc_call::RpcCallValue,
};

//---------------------------------------------------------------------------------------------------- Macro
/// Adds a (de)serialization doc-test to a type in `json.rs`.
///
/// It expects a const string from `cuprate_test_utils::rpc::data`
/// and the expected value it should (de)serialize into/from.
///
/// It tests that the provided const JSON string can properly
/// (de)serialize into the expected value.
///
/// See below for example usage. This macro is only used in this file.
macro_rules! serde_doc_test {
    (
        // `const` string from `cuprate_test_utils::rpc::data`
        //  v
        $cuprate_test_utils_rpc_const:ident => $expected:expr
        //                                     ^
        //                     Expected value as an expression
    ) => {
        paste::paste! {
            concat!(
                "```rust\n",
                "use cuprate_test_utils::rpc::data::json::*;\n",
                "use cuprate_rpc_types::{misc::*, base::*, json::*};\n",
                "use serde_json::{Value, from_str, from_value};\n",
                "\n",
                "// The expected data.\n",
                "let expected = ",
                stringify!($expected),
                ";\n",
                "\n",
                "// Assert it can be turned into a JSON value.\n",
                "let value = from_str::<Value>(",
                stringify!($cuprate_test_utils_rpc_const),
                ").unwrap();\n",
                "let Value::Object(map) = value else {\n",
                "    panic!();\n",
                "};\n",
                "\n",
                "// If a request...\n",
                "if let Some(params) = map.get(\"params\") {\n",
                "    let response = from_value::<",
                stringify!([<$cuprate_test_utils_rpc_const:camel>]),
                ">(params.clone()).unwrap();\n",
                "    assert_eq!(response, expected);\n",
                "    return;\n",
                "}\n",
                "\n",
                "// Else, if a response...\n",
                "let result = map.get(\"result\").unwrap().clone();\n",
                "let response = from_value::<",
                stringify!([<$cuprate_test_utils_rpc_const:camel>]),
                ">(result.clone()).unwrap();\n",
                "assert_eq!(response, expected);\n",
                "```\n",
            )
        }
    };
}

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
    //
    // After the type name, 2 optional idents are allowed:
    // - `restricted`
    // - `empty`
    //
    // These have to be within `()` and will affect the
    // [`crate::RpcCall`] implementation on the request type.
    //
    // This type is not either restricted or empty so nothing is
    // here, but the correct syntax is shown in a comment below:
    GetBlockTemplate /* (restricted, empty) */,

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
    #[doc = serde_doc_test!(
        // ^ This is a macro that adds a doc-test to this type.
        // It is optional but it is added to nearly all types.
        // The syntax is:
        // `$const` => `$expected`
        // where `$const` is a `const` string from
        // `cuprate_test_utils::rpc::data` and `$expected` is an
        // actual expression that the string _should_ (de)serialize into/from.
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
    #[doc = serde_doc_test!(
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
    GetBlockCount (empty),

    // There are no request fields specified,
    // this will cause the macro to generate a
    // type alias to `()` instead of a `struct`.
    Request {},

    #[doc = serde_doc_test!(
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

    #[doc = serde_doc_test!(
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

    #[doc = serde_doc_test!(
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

    #[doc = serde_doc_test!(
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

    GenerateBlocks (restricted),

    #[doc = serde_doc_test!(
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

    #[doc = serde_doc_test!(
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

    #[doc = serde_doc_test!(
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
    #[doc = serde_doc_test!(
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

    #[doc = serde_doc_test!(
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
    #[doc = serde_doc_test!(
        GET_BLOCK_HEADER_BY_HEIGHT_REQUEST => GetBlockHeaderByHeightRequest {
            height: 912345,
            fill_pow_hash: false,
        }
    )]
    Request {
        height: u64,
        fill_pow_hash: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
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

define_request_and_response! {
    get_block_headers_range,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1756..=1783,

    GetBlockHeadersRange,

    #[derive(Copy)]
    #[doc = serde_doc_test!(
        GET_BLOCK_HEADERS_RANGE_REQUEST => GetBlockHeadersRangeRequest {
            start_height: 1545999,
            end_height: 1546000,
            fill_pow_hash: false,
        }
    )]
    Request {
        start_height: u64,
        end_height: u64,
        fill_pow_hash: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        GET_BLOCK_HEADERS_RANGE_RESPONSE => GetBlockHeadersRangeResponse {
            base: AccessResponseBase::ok(),
            headers: vec![
                BlockHeader {
                    block_size: 301413,
                    block_weight: 301413,
                    cumulative_difficulty: 13185267971483472,
                    cumulative_difficulty_top64: 0,
                    depth: 1649024,
                    difficulty: 134636057921,
                    difficulty_top64: 0,
                    hash: "86d1d20a40cefcf3dd410ff6967e0491613b77bf73ea8f1bf2e335cf9cf7d57a".into(),
                    height: 1545999,
                    long_term_weight: 301413,
                    major_version: 6,
                    miner_tx_hash: "9909c6f8a5267f043c3b2b079fb4eacc49ef9c1dee1c028eeb1a259b95e6e1d9".into(),
                    minor_version: 6,
                    nonce: 3246403956,
                    num_txes: 20,
                    orphan_status: false,
                    pow_hash: "".into(),
                    prev_hash: "0ef6e948f77b8f8806621003f5de24b1bcbea150bc0e376835aea099674a5db5".into(),
                    reward: 5025593029981,
                    timestamp: 1523002893,
                    wide_cumulative_difficulty: "0x2ed7ee6db56750".into(),
                    wide_difficulty: "0x1f58ef3541".into()
                },
                BlockHeader {
                    block_size: 13322,
                    block_weight: 13322,
                    cumulative_difficulty: 13185402687569710,
                    cumulative_difficulty_top64: 0,
                    depth: 1649023,
                    difficulty: 134716086238,
                    difficulty_top64: 0,
                    hash: "b408bf4cfcd7de13e7e370c84b8314c85b24f0ba4093ca1d6eeb30b35e34e91a".into(),
                    height: 1546000,
                    long_term_weight: 13322,
                    major_version: 7,
                    miner_tx_hash: "7f749c7c64acb35ef427c7454c45e6688781fbead9bbf222cb12ad1a96a4e8f6".into(),
                    minor_version: 7,
                    nonce: 3737164176,
                    num_txes: 1,
                    orphan_status: false,
                    pow_hash: "".into(),
                    prev_hash: "86d1d20a40cefcf3dd410ff6967e0491613b77bf73ea8f1bf2e335cf9cf7d57a".into(),
                    reward: 4851952181070,
                    timestamp: 1523002931,
                    wide_cumulative_difficulty: "0x2ed80dcb69bf2e".into(),
                    wide_difficulty: "0x1f5db457de".into()
                }
            ],
        }
    )]
    AccessResponseBase {
        headers: Vec<BlockHeader>,
    }
}

define_request_and_response! {
    get_block,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1298..=1313,
    GetBlock,

    #[doc = serde_doc_test!(
        GET_BLOCK_REQUEST => GetBlockRequest {
            height: 2751506,
            hash: String::default(),
            fill_pow_hash: false,
        }
    )]
    Request {
        // `monerod` has both `hash` and `height` fields.
        // In the RPC handler, if `hash.is_empty()`, it will use it, else, it uses `height`.
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2674>
        hash: String = default_string(), "default_string",
        height: u64 = default_height(), "default_height",
        fill_pow_hash: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        GET_BLOCK_RESPONSE => GetBlockResponse {
            base: AccessResponseBase::ok(),
            blob: "1010c58bab9b06b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7807e07f502cef8a70101ff92f8a7010180e0a596bb1103d7cbf826b665d7a532c316982dc8dbc24f285cbc18bbcc27c7164cd9b3277a85d034019f629d8b36bd16a2bfce3ea80c31dc4d8762c67165aec21845494e32b7582fe00211000000297a787a000000000000000000000000".into(),
            block_header: BlockHeader {
                block_size: 106,
                block_weight: 106,
                cumulative_difficulty: 236046001376524168,
                cumulative_difficulty_top64: 0,
                depth: 443517,
                difficulty: 313732272488,
                difficulty_top64: 0,
                hash: "43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428".into(),
                height: 2751506,
                long_term_weight: 176470,
                major_version: 16,
                miner_tx_hash: "e49b854c5f339d7410a77f2a137281d8042a0ffc7ef9ab24cd670b67139b24cd".into(),
                minor_version: 16,
                nonce: 4110909056,
                num_txes: 0,
                orphan_status: false,
                pow_hash: "".into(),
                prev_hash: "b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7".into(),
                reward: 600000000000,
                timestamp: 1667941829,
                wide_cumulative_difficulty: "0x3469a966eb2f788".into(),
                wide_difficulty: "0x490be69168".into()
            },
            json: "{\n  \"major_version\": 16, \n  \"minor_version\": 16, \n  \"timestamp\": 1667941829, \n  \"prev_id\": \"b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7\", \n  \"nonce\": 4110909056, \n  \"miner_tx\": {\n    \"version\": 2, \n    \"unlock_time\": 2751566, \n    \"vin\": [ {\n        \"gen\": {\n          \"height\": 2751506\n        }\n      }\n    ], \n    \"vout\": [ {\n        \"amount\": 600000000000, \n        \"target\": {\n          \"tagged_key\": {\n            \"key\": \"d7cbf826b665d7a532c316982dc8dbc24f285cbc18bbcc27c7164cd9b3277a85\", \n            \"view_tag\": \"d0\"\n          }\n        }\n      }\n    ], \n    \"extra\": [ 1, 159, 98, 157, 139, 54, 189, 22, 162, 191, 206, 62, 168, 12, 49, 220, 77, 135, 98, 198, 113, 101, 174, 194, 24, 69, 73, 78, 50, 183, 88, 47, 224, 2, 17, 0, 0, 0, 41, 122, 120, 122, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0\n    ], \n    \"rct_signatures\": {\n      \"type\": 0\n    }\n  }, \n  \"tx_hashes\": [ ]\n}".into(),
            miner_tx_hash: "e49b854c5f339d7410a77f2a137281d8042a0ffc7ef9ab24cd670b67139b24cd".into(),
            tx_hashes: vec![],
        }
    )]
    AccessResponseBase {
        blob: String,
        block_header: BlockHeader,
        json: String, // FIXME: this should be defined in a struct, it has many fields.
        miner_tx_hash: String,
        tx_hashes: Vec<String> = default_vec::<String>(), "default_vec",
    }
}

define_request_and_response! {
    get_connections,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1734..=1754,

    GetConnections (restricted, empty),

    Request {},

    #[doc = serde_doc_test!(
        GET_CONNECTIONS_RESPONSE => GetConnectionsResponse {
            base: ResponseBase::ok(),
            connections: vec![
                ConnectionInfo {
                    address: "3evk3kezfjg44ma6tvesy7rbxwwpgpympj45xar5fo4qajrsmkoaqdqd.onion:18083".into(),
                    address_type: 4,
                    avg_download: 0,
                    avg_upload: 0,
                    connection_id: "22ef856d0f1d44cc95e84fecfd065fe2".into(),
                    current_download: 0,
                    current_upload: 0,
                    height: 3195026,
                    host: "3evk3kezfjg44ma6tvesy7rbxwwpgpympj45xar5fo4qajrsmkoaqdqd.onion".into(),
                    incoming: false,
                    ip: "".into(),
                    live_time: 76651,
                    local_ip: false,
                    localhost: false,
                    peer_id: "0000000000000001".into(),
                    port: "".into(),
                    pruning_seed: 0,
                    recv_count: 240328,
                    recv_idle_time: 34,
                    rpc_credits_per_hash: 0,
                    rpc_port: 0,
                    send_count: 3406572,
                    send_idle_time: 30,
                    state: "normal".into(),
                    support_flags: 0
                },
                ConnectionInfo {
                    address: "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion:18083".into(),
                    address_type: 4,
                    avg_download: 0,
                    avg_upload: 0,
                    connection_id: "c7734e15936f485a86d2b0534f87e499".into(),
                    current_download: 0,
                    current_upload: 0,
                    height: 3195024,
                    host: "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion".into(),
                    incoming: false,
                    ip: "".into(),
                    live_time: 76755,
                    local_ip: false,
                    localhost: false,
                    peer_id: "0000000000000001".into(),
                    port: "".into(),
                    pruning_seed: 389,
                    recv_count: 237657,
                    recv_idle_time: 120,
                    rpc_credits_per_hash: 0,
                    rpc_port: 0,
                    send_count: 3370566,
                    send_idle_time: 120,
                    state: "normal".into(),
                    support_flags: 0
                }
            ],
        }
    )]
    ResponseBase {
        // FIXME: This is a `std::list` in `monerod` because...?
        connections: Vec<ConnectionInfo>,
    }
}

define_request_and_response! {
    get_info,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 693..=789,
    GetInfo (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_INFO_RESPONSE => GetInfoResponse {
            base: AccessResponseBase::ok(),
            adjusted_time: 1721245289,
            alt_blocks_count: 16,
            block_size_limit: 600000,
            block_size_median: 300000,
            block_weight_limit: 600000,
            block_weight_median: 300000,
            bootstrap_daemon_address: "".into(),
            busy_syncing: false,
            cumulative_difficulty: 366127702242611947,
            cumulative_difficulty_top64: 0,
            database_size: 235169075200,
            difficulty: 280716748706,
            difficulty_top64: 0,
            free_space: 30521749504,
            grey_peerlist_size: 4996,
            height: 3195028,
            height_without_bootstrap: 3195028,
            incoming_connections_count: 62,
            mainnet: true,
            nettype: "mainnet".into(),
            offline: false,
            outgoing_connections_count: 1143,
            restricted: false,
            rpc_connections_count: 1,
            stagenet: false,
            start_time: 1720462427,
            synchronized: true,
            target: 120,
            target_height: 0,
            testnet: false,
            top_block_hash: "bdf06d18ed1931a8ee62654e9b6478cc459bc7072628b8e36f4524d339552946".into(),
            tx_count: 43205750,
            tx_pool_size: 12,
            update_available: false,
            version: "0.18.3.3-release".into(),
            was_bootstrap_ever_used: false,
            white_peerlist_size: 1000,
            wide_cumulative_difficulty: "0x514bf349299d2eb".into(),
            wide_difficulty: "0x415c05a7a2".into()
        }
    )]
    AccessResponseBase {
        adjusted_time: u64,
        alt_blocks_count: u64,
        block_size_limit: u64,
        block_size_median: u64,
        block_weight_limit: u64,
        block_weight_median: u64,
        bootstrap_daemon_address: String,
        busy_syncing: bool,
        cumulative_difficulty_top64: u64,
        cumulative_difficulty: u64,
        database_size: u64,
        difficulty_top64: u64,
        difficulty: u64,
        free_space: u64,
        grey_peerlist_size: u64,
        height: u64,
        height_without_bootstrap: u64,
        incoming_connections_count: u64,
        mainnet: bool,
        nettype: String,
        offline: bool,
        outgoing_connections_count: u64,
        restricted: bool,
        rpc_connections_count: u64,
        stagenet: bool,
        start_time: u64,
        synchronized: bool,
        target_height: u64,
        target: u64,
        testnet: bool,
        top_block_hash: String,
        tx_count: u64,
        tx_pool_size: u64,
        update_available: bool,
        version: String,
        was_bootstrap_ever_used: bool,
        white_peerlist_size: u64,
        wide_cumulative_difficulty: String,
        wide_difficulty: String,
    }
}

define_request_and_response! {
    hard_fork_info,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1958..=1995,
    HardForkInfo,

    #[doc = serde_doc_test!(
        HARD_FORK_INFO => HardForkInfo {
            version: 16,
        }
    )]
    #[derive(Copy)]
    Request {
        version: u8,
    },

    #[doc = serde_doc_test!(
        HARD_FORK_INFO_RESPONSE => HardForkInfoResponse {
            base: AccessResponseBase::ok(),
            earliest_height: 2689608,
            enabled: true,
            state: 0,
            threshold: 0,
            version: 3,
            votes: 10080,
            voting: 3,
            window: 10080
        }
    )]
    AccessResponseBase {
        earliest_height: u64,
        enabled: bool,
        state: u32,
        threshold: u32,
        version: u8,
        votes: u32,
        voting: u8,
        window: u32,
    }
}

define_request_and_response! {
    set_bans,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2032..=2067,

    SetBans (restricted),

    #[doc = serde_doc_test!(
        SET_BANS_REQUEST => SetBansRequest {
            bans: vec![ SetBan {
                host: "192.168.1.51".into(),
                ip: 0,
                ban: true,
                seconds: 30
            }]
        }
    )]
    Request {
        bans: Vec<SetBan>,
    },

    #[doc = serde_doc_test!(
        SET_BANS_RESPONSE => SetBansResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    get_bans,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1997..=2030,
    GetBans (restricted, empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_BANS_RESPONSE => GetBansResponse {
            base: ResponseBase::ok(),
            bans: vec![
                GetBan {
                    host: "104.248.206.131".into(),
                    ip: 2211379304,
                    seconds: 689754
                },
                GetBan {
                    host: "209.222.252.0/24".into(),
                    ip: 0,
                    seconds: 689754
                }
            ]
        }
    )]
    ResponseBase {
        bans: Vec<GetBan>,
    }
}

define_request_and_response! {
    banned,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2069..=2094,

    Banned (restricted),

    #[doc = serde_doc_test!(
        BANNED_REQUEST => BannedRequest {
            address: "95.216.203.255".into(),
        }
    )]
    Request {
        address: String,
    },

    #[doc = serde_doc_test!(
        BANNED_RESPONSE => BannedResponse {
            banned: true,
            seconds: 689655,
            status: Status::Ok,
        }
    )]
    Response {
        banned: bool,
        seconds: u32,
        status: Status,
    }
}

define_request_and_response! {
    flush_txpool,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2096..=2116,

    FlushTransactionPool (restricted),

    #[doc = serde_doc_test!(
        FLUSH_TRANSACTION_POOL_REQUEST => FlushTransactionPoolRequest {
            txids: vec!["dc16fa8eaffe1484ca9014ea050e13131d3acf23b419f33bb4cc0b32b6c49308".into()],
        }
    )]
    Request {
        txids: Vec<String> = default_vec::<String>(), "default_vec",
    },

    #[doc = serde_doc_test!(
        FLUSH_TRANSACTION_POOL_RESPONSE => FlushTransactionPoolResponse {
            status: Status::Ok,
        }
    )]
    #[repr(transparent)]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    get_output_histogram,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2118..=2168,
    GetOutputHistogram,

    #[doc = serde_doc_test!(
        GET_OUTPUT_HISTOGRAM_REQUEST => GetOutputHistogramRequest {
            amounts: vec![20000000000],
            min_count: 0,
            max_count: 0,
            unlocked: false,
            recent_cutoff: 0,
        }
    )]
    Request {
        amounts: Vec<u64>,
        min_count: u64 = default_zero::<u64>(), "default_zero",
        max_count: u64 = default_zero::<u64>(), "default_zero",
        unlocked: bool = default_false(), "default_false",
        recent_cutoff: u64 = default_zero::<u64>(), "default_zero",
    },

    #[doc = serde_doc_test!(
        GET_OUTPUT_HISTOGRAM_RESPONSE => GetOutputHistogramResponse {
            base: AccessResponseBase::ok(),
            histogram: vec![HistogramEntry {
                amount: 20000000000,
                recent_instances: 0,
                total_instances: 381490,
                unlocked_instances: 0
            }]
        }
    )]
    AccessResponseBase {
        histogram: Vec<HistogramEntry>,
    }
}

define_request_and_response! {
    get_coinbase_tx_sum,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2213..=2248,

    GetCoinbaseTxSum (restricted),

    #[doc = serde_doc_test!(
        GET_COINBASE_TX_SUM_REQUEST => GetCoinbaseTxSumRequest {
            height: 1563078,
            count: 2
        }
    )]
    Request {
        height: u64,
        count: u64,
    },

    #[doc = serde_doc_test!(
        GET_COINBASE_TX_SUM_RESPONSE => GetCoinbaseTxSumResponse {
            base: AccessResponseBase::ok(),
            emission_amount: 9387854817320,
            emission_amount_top64: 0,
            fee_amount: 83981380000,
            fee_amount_top64: 0,
            wide_emission_amount: "0x889c7c06828".into(),
            wide_fee_amount: "0x138dae29a0".into()
        }
    )]
    AccessResponseBase {
        emission_amount: u64,
        emission_amount_top64: u64,
        fee_amount: u64,
        fee_amount_top64: u64,
        wide_emission_amount: String,
        wide_fee_amount: String,
    }
}

define_request_and_response! {
    get_version,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2170..=2211,

    GetVersion (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_VERSION_RESPONSE => GetVersionResponse {
            base: ResponseBase::ok(),
            current_height: 3195051,
            hard_forks: vec![
                HardforkEntry {
                    height: 1,
                    hf_version: 1
                },
                HardforkEntry {
                    height: 1009827,
                    hf_version: 2
                },
                HardforkEntry {
                    height: 1141317,
                    hf_version: 3
                },
                HardforkEntry {
                    height: 1220516,
                    hf_version: 4
                },
                HardforkEntry {
                    height: 1288616,
                    hf_version: 5
                },
                HardforkEntry {
                    height: 1400000,
                    hf_version: 6
                },
                HardforkEntry {
                    height: 1546000,
                    hf_version: 7
                },
                HardforkEntry {
                    height: 1685555,
                    hf_version: 8
                },
                HardforkEntry {
                    height: 1686275,
                    hf_version: 9
                },
                HardforkEntry {
                    height: 1788000,
                    hf_version: 10
                },
                HardforkEntry {
                    height: 1788720,
                    hf_version: 11
                },
                HardforkEntry {
                    height: 1978433,
                    hf_version: 12
                },
                HardforkEntry {
                    height: 2210000,
                    hf_version: 13
                },
                HardforkEntry {
                    height: 2210720,
                    hf_version: 14
                },
                HardforkEntry {
                    height: 2688888,
                    hf_version: 15
                },
                HardforkEntry {
                    height: 2689608,
                    hf_version: 16
                }
            ],
            release: true,
            version: 196621,
            target_height: 0,
        }
    )]
    ResponseBase {
        version: u32,
        release: bool,
        current_height: u64 = default_zero::<u64>(), "default_zero",
        target_height: u64 = default_zero::<u64>(), "default_zero",
        hard_forks: Vec<HardforkEntry> = default_vec(), "default_vec",
    }
}

define_request_and_response! {
    get_fee_estimate,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2250..=2277,
    GetFeeEstimate (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_FEE_ESTIMATE_RESPONSE => GetFeeEstimateResponse {
            base: AccessResponseBase::ok(),
            fee: 20000,
            fees: vec![20000,80000,320000,4000000],
            quantization_mask: 10000,
        }
    )]
    AccessResponseBase {
        fee: u64,
        fees: Vec<u64>,
        quantization_mask: u64 = default_one::<u64>(), "default_one",
    }
}

define_request_and_response! {
    get_alternate_chains,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2279..=2310,
    GetAlternateChains (restricted, empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_ALTERNATE_CHAINS_RESPONSE => GetAlternateChainsResponse {
            base: ResponseBase::ok(),
            chains: vec![
                ChainInfo {
                    block_hash: "4826c7d45d7cf4f02985b5c405b0e5d7f92c8d25e015492ce19aa3b209295dce".into(),
                    block_hashes: vec!["4826c7d45d7cf4f02985b5c405b0e5d7f92c8d25e015492ce19aa3b209295dce".into()],
                    difficulty: 357404825113208373,
                    difficulty_top64: 0,
                    height: 3167471,
                    length: 1,
                    main_chain_parent_block: "69b5075ea627d6ba06b1c30b7e023884eeaef5282cf58ec847dab838ddbcdd86".into(),
                    wide_difficulty: "0x4f5c1cb79e22635".into(),
                },
                ChainInfo {
                    block_hash: "33ee476f5a1c5b9d889274cbbe171f5e0112df7ed69021918042525485deb401".into(),
                    block_hashes: vec!["33ee476f5a1c5b9d889274cbbe171f5e0112df7ed69021918042525485deb401".into()],
                    difficulty: 354736121711617293,
                    difficulty_top64: 0,
                    height: 3157465,
                    length: 1,
                    main_chain_parent_block: "fd522fcc4cefe5c8c0e5c5600981b3151772c285df3a4e38e5c4011cf466d2cb".into(),
                    wide_difficulty: "0x4ec469f8b9ee50d".into(),
                }
            ],
        }
    )]
    ResponseBase {
        chains: Vec<ChainInfo>,
    }
}

define_request_and_response! {
    relay_tx,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2361..=2381,

    RelayTx (restricted),

    #[doc = serde_doc_test!(
        RELAY_TX_REQUEST => RelayTxRequest {
            txids: vec!["9fd75c429cbe52da9a52f2ffc5fbd107fe7fd2099c0d8de274dc8a67e0c98613".into()]
        }
    )]
    Request {
        txids: Vec<String>,
    },

    #[doc = serde_doc_test!(
        RELAY_TX_RESPONSE => RelayTxResponse {
            status: Status::Ok,
        }
    )]
    #[repr(transparent)]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    sync_info,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2383..=2443,

    SyncInfo (restricted, empty),

    Request {},

    #[doc = serde_doc_test!(
        SYNC_INFO_RESPONSE => SyncInfoResponse {
            base: AccessResponseBase::ok(),
            height: 3195157,
            next_needed_pruning_seed: 0,
            overview: "[]".into(),
            spans: vec![],
            peers: vec![
                SyncInfoPeer {
                    info: ConnectionInfo {
                        address: "142.93.128.65:44986".into(),
                        address_type: 1,
                        avg_download: 1,
                        avg_upload: 1,
                        connection_id: "a5803c4c2dac49e7b201dccdef54c862".into(),
                        current_download: 2,
                        current_upload: 1,
                        height: 3195157,
                        host: "142.93.128.65".into(),
                        incoming: true,
                        ip: "142.93.128.65".into(),
                        live_time: 18,
                        local_ip: false,
                        localhost: false,
                        peer_id: "6830e9764d3e5687".into(),
                        port: "44986".into(),
                        pruning_seed: 0,
                        recv_count: 20340,
                        recv_idle_time: 0,
                        rpc_credits_per_hash: 0,
                        rpc_port: 18089,
                        send_count: 32235,
                        send_idle_time: 6,
                        state: "normal".into(),
                        support_flags: 1
                    }
                },
                SyncInfoPeer {
                    info: ConnectionInfo {
                        address: "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion:18083".into(),
                        address_type: 4,
                        avg_download: 0,
                        avg_upload: 0,
                        connection_id: "277f7c821bc546878c8bd29977e780f5".into(),
                        current_download: 0,
                        current_upload: 0,
                        height: 3195157,
                        host: "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion".into(),
                        incoming: false,
                        ip: "".into(),
                        live_time: 2246,
                        local_ip: false,
                        localhost: false,
                        peer_id: "0000000000000001".into(),
                        port: "".into(),
                        pruning_seed: 389,
                        recv_count: 65164,
                        recv_idle_time: 15,
                        rpc_credits_per_hash: 0,
                        rpc_port: 0,
                        send_count: 99120,
                        send_idle_time: 15,
                        state: "normal".into(),
                        support_flags: 0
                    }
                }
            ],
            target_height: 0,
        }
    )]
    AccessResponseBase {
        height: u64,
        next_needed_pruning_seed: u32,
        overview: String,
        // FIXME: This is a `std::list` in `monerod` because...?
        peers: Vec<SyncInfoPeer> = default_vec::<SyncInfoPeer>(), "default_vec",
        // FIXME: This is a `std::list` in `monerod` because...?
        spans: Vec<Span> = default_vec::<Span>(), "default_vec",
        target_height: u64,
    }
}

define_request_and_response! {
    get_txpool_backlog,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1637..=1664,
    GetTransactionPoolBacklog (empty),
    Request {},

    // TODO: enable test after binary string impl.
    // #[doc = serde_doc_test!(
    //     GET_TRANSACTION_POOL_BACKLOG_RESPONSE => GetTransactionPoolBacklogResponse {
    //         base: ResponseBase::ok(),
    //         backlog: "...Binary...".into(),
    //     }
    // )]
    ResponseBase {
        // TODO: this is a [`BinaryString`].
        backlog: Vec<TxBacklogEntry>,
    }
}

define_request_and_response! {
    get_output_distribution,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2445..=2520,

    /// This type is also used in the (undocumented)
    /// [`/get_output_distribution.bin`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L138)
    /// binary endpoint.
    GetOutputDistribution,

    #[doc = serde_doc_test!(
        GET_OUTPUT_DISTRIBUTION_REQUEST => GetOutputDistributionRequest {
            amounts: vec![628780000],
            from_height: 1462078,
            binary: true,
            compress: false,
            cumulative: false,
            to_height: 0,
        }
    )]
    Request {
        amounts: Vec<u64>,
        binary: bool = default_true(), "default_true",
        compress: bool = default_false(), "default_false",
        cumulative: bool = default_false(), "default_false",
        from_height: u64 = default_zero::<u64>(), "default_zero",
        to_height: u64 = default_zero::<u64>(), "default_zero",
    },

    // TODO: enable test after binary string impl.
    // #[doc = serde_doc_test!(
    //     GET_OUTPUT_DISTRIBUTION_RESPONSE => GetOutputDistributionResponse {
    //         base: AccessResponseBase::ok(),
    //         distributions: vec![Distribution::Uncompressed(DistributionUncompressed {
    //             start_height: 1462078,
    //             base: 0,
    //             distribution: vec![],
    //             amount: 2628780000,
    //             binary: true,
    //         })],
    //     }
    // )]
    AccessResponseBase {
        distributions: Vec<Distribution>,
    }
}

define_request_and_response! {
    get_miner_data,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 996..=1044,
    GetMinerData (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_MINER_DATA_RESPONSE => GetMinerDataResponse {
            base: ResponseBase::ok(),
            already_generated_coins: 18186022843595960691,
            difficulty: "0x48afae42de".into(),
            height: 2731375,
            major_version: 16,
            median_weight: 300000,
            prev_id: "78d50c5894d187c4946d54410990ca59a75017628174a9e8c7055fa4ca5c7c6d".into(),
            seed_hash: "a6b869d50eca3a43ec26fe4c369859cf36ae37ce6ecb76457d31ffeb8a6ca8a6".into(),
            tx_backlog: vec![
                GetMinerDataTxBacklogEntry {
                    fee: 30700000,
                    id: "9868490d6bb9207fdd9cf17ca1f6c791b92ca97de0365855ea5c089f67c22208".into(),
                    weight: 1535
                },
                GetMinerDataTxBacklogEntry {
                    fee: 44280000,
                    id: "b6000b02bbec71e18ad704bcae09fb6e5ae86d897ced14a718753e76e86c0a0a".into(),
                    weight: 2214
                },
            ],
        }
    )]
    ResponseBase {
        major_version: u8,
        height: u64,
        prev_id: String,
        seed_hash: String,
        difficulty: String,
        median_weight: u64,
        already_generated_coins: u64,
        tx_backlog: Vec<GetMinerDataTxBacklogEntry>,
    }
}

define_request_and_response! {
    prune_blockchain,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2747..=2772,

    PruneBlockchain (restricted),

    #[derive(Copy)]
    #[doc = serde_doc_test!(
        PRUNE_BLOCKCHAIN_REQUEST => PruneBlockchainRequest {
            check: true
        }
    )]
    Request {
        check: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        PRUNE_BLOCKCHAIN_RESPONSE => PruneBlockchainResponse {
            base: ResponseBase::ok(),
            pruned: true,
            pruning_seed: 387,
        }
    )]
    ResponseBase {
        pruned: bool,
        pruning_seed: u32,
    }
}

define_request_and_response! {
    calc_pow,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1046..=1066,

    CalcPow (restricted),

    #[doc = serde_doc_test!(
        CALC_POW_REQUEST => CalcPowRequest {
            major_version: 14,
            height: 2286447,
            block_blob: "0e0ed286da8006ecdc1aab3033cf1716c52f13f9d8ae0051615a2453643de94643b550d543becd0000000002abc78b0101ffefc68b0101fcfcf0d4b422025014bb4a1eade6622fd781cb1063381cad396efa69719b41aa28b4fce8c7ad4b5f019ce1dc670456b24a5e03c2d9058a2df10fec779e2579753b1847b74ee644f16b023c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000051399a1bc46a846474f5b33db24eae173a26393b976054ee14f9feefe99925233802867097564c9db7a36af5bb5ed33ab46e63092bd8d32cef121608c3258edd55562812e21cc7e3ac73045745a72f7d74581d9a0849d6f30e8b2923171253e864f4e9ddea3acb5bc755f1c4a878130a70c26297540bc0b7a57affb6b35c1f03d8dbd54ece8457531f8cba15bb74516779c01193e212050423020e45aa2c15dcb".into(),
            seed_hash: "d432f499205150873b2572b5f033c9c6e4b7c6f3394bd2dd93822cd7085e7307".into(),
        }
    )]
    Request {
        major_version: u8,
        height: u64,
        block_blob: String,
        seed_hash: String,
    },

    #[doc = serde_doc_test!(
        CALC_POW_RESPONSE => CalcPowResponse {
            pow_hash: "d0402d6834e26fb94a9ce38c6424d27d2069896a9b8b1ce685d79936bca6e0a8".into(),
        }
    )]
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        pow_hash: String,
    }
}

define_request_and_response! {
    flush_cache,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2774..=2796,

    FlushCache (restricted),

    #[derive(Copy)]
    #[doc = serde_doc_test!(
        FLUSH_CACHE_REQUEST => FlushCacheRequest {
            bad_txs: true,
            bad_blocks: true
        }
    )]
    Request {
        bad_txs: bool = default_false(), "default_false",
        bad_blocks: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        FLUSH_CACHE_RESPONSE => FlushCacheResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    add_aux_pow,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1068..=1112,

    AddAuxPow,

    #[doc = serde_doc_test!(
        ADD_AUX_POW_REQUEST => AddAuxPowRequest {
            blocktemplate_blob: "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
            aux_pow: vec![AuxPow {
                id: "3200b4ea97c3b2081cd4190b58e49572b2319fed00d030ad51809dff06b5d8c8".into(),
                hash: "7b35762de164b20885e15dbe656b1138db06bb402fa1796f5765a23933d8859a".into()
            }]
        }
    )]
    Request {
        blocktemplate_blob: String,
        aux_pow: Vec<AuxPow>,
    },

    #[doc = serde_doc_test!(
        ADD_AUX_POW_RESPONSE => AddAuxPowResponse {
            base: ResponseBase::ok(),
            aux_pow: vec![AuxPow {
                hash: "7b35762de164b20885e15dbe656b1138db06bb402fa1796f5765a23933d8859a".into(),
                id: "3200b4ea97c3b2081cd4190b58e49572b2319fed00d030ad51809dff06b5d8c8".into(),
            }],
            blockhashing_blob: "1010ee97e2a106e9f8ebe8887e5b609949ac8ea6143e560ed13552b110cb009b21f0cfca1eaccf00000000b2685c1283a646bc9020c758daa443be145b7370ce5a6efacb3e614117032e2c22".into(),
            blocktemplate_blob: "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
            merkle_root: "7b35762de164b20885e15dbe656b1138db06bb402fa1796f5765a23933d8859a".into(),
            merkle_tree_depth: 0,
        }
    )]
    ResponseBase {
      blocktemplate_blob: String,
      blockhashing_blob: String,
      merkle_root: String,
      merkle_tree_depth: u64,
      aux_pow: Vec<AuxPow>,
    }
}

define_request_and_response! {
    UNDOCUMENTED_METHOD,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2798..=2823,

    GetTxIdsLoose,

    Request {
        txid_template: String,
        num_matching_bits: u32,
    },
    ResponseBase {
        txids: Vec<String>,
    }
}

//---------------------------------------------------------------------------------------------------- Request
/// JSON-RPC requests.
///
/// This enum contains all [`crate::json`] requests.
///
/// See also: [`JsonRpcResponse`].
///
/// TODO: document and test (de)serialization behavior after figuring out `method/params`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "snake_case", tag = "method", content = "params")
)]
#[allow(missing_docs)]
pub enum JsonRpcRequest {
    GetBlockCount(GetBlockCountRequest),
    OnGetBlockHash(OnGetBlockHashRequest),
    SubmitBlock(SubmitBlockRequest),
    GenerateBlocks(GenerateBlocksRequest),
    GetLastBlockHeader(GetLastBlockHeaderRequest),
    GetBlockHeaderByHash(GetBlockHeaderByHashRequest),
    GetBlockHeaderByHeight(GetBlockHeaderByHeightRequest),
    GetBlockHeadersRange(GetBlockHeadersRangeRequest),
    GetBlock(GetBlockRequest),
    GetConnections(GetConnectionsRequest),
    GetInfo(GetInfoRequest),
    HardForkInfo(HardForkInfoRequest),
    SetBans(SetBansRequest),
    GetBans(GetBansRequest),
    Banned(BannedRequest),
    FlushTransactionPool(FlushTransactionPoolRequest),
    GetOutputHistogram(GetOutputHistogramRequest),
    GetCoinbaseTxSum(GetCoinbaseTxSumRequest),
    GetVersion(GetVersionRequest),
    GetFeeEstimate(GetFeeEstimateRequest),
    GetAlternateChains(GetAlternateChainsRequest),
    RelayTx(RelayTxRequest),
    SyncInfo(SyncInfoRequest),
    GetTransactionPoolBacklog(GetTransactionPoolBacklogRequest),
    GetMinerData(GetMinerDataRequest),
    PruneBlockchain(PruneBlockchainRequest),
    CalcPow(CalcPowRequest),
    FlushCache(FlushCacheRequest),
    AddAuxPow(AddAuxPowRequest),
    GetTxIdsLoose(GetTxIdsLooseRequest),
}

impl RpcCallValue for JsonRpcRequest {
    fn is_restricted(&self) -> bool {
        match self {
            Self::GetBlockCount(x) => x.is_restricted(),
            Self::OnGetBlockHash(x) => x.is_restricted(),
            Self::SubmitBlock(x) => x.is_restricted(),
            Self::GetLastBlockHeader(x) => x.is_restricted(),
            Self::GetBlockHeaderByHash(x) => x.is_restricted(),
            Self::GetBlockHeaderByHeight(x) => x.is_restricted(),
            Self::GetBlockHeadersRange(x) => x.is_restricted(),
            Self::GetBlock(x) => x.is_restricted(),
            Self::GetInfo(x) => x.is_restricted(),
            Self::HardForkInfo(x) => x.is_restricted(),
            Self::GetOutputHistogram(x) => x.is_restricted(),
            Self::GetVersion(x) => x.is_restricted(),
            Self::GetFeeEstimate(x) => x.is_restricted(),
            Self::GetTransactionPoolBacklog(x) => x.is_restricted(),
            Self::GetMinerData(x) => x.is_restricted(),
            Self::AddAuxPow(x) => x.is_restricted(),
            Self::GetTxIdsLoose(x) => x.is_restricted(),
            Self::GenerateBlocks(x) => x.is_restricted(),
            Self::GetConnections(x) => x.is_restricted(),
            Self::SetBans(x) => x.is_restricted(),
            Self::GetBans(x) => x.is_restricted(),
            Self::Banned(x) => x.is_restricted(),
            Self::FlushTransactionPool(x) => x.is_restricted(),
            Self::GetCoinbaseTxSum(x) => x.is_restricted(),
            Self::GetAlternateChains(x) => x.is_restricted(),
            Self::RelayTx(x) => x.is_restricted(),
            Self::SyncInfo(x) => x.is_restricted(),
            Self::PruneBlockchain(x) => x.is_restricted(),
            Self::CalcPow(x) => x.is_restricted(),
            Self::FlushCache(x) => x.is_restricted(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::GetBlockCount(x) => x.is_empty(),
            Self::OnGetBlockHash(x) => x.is_empty(),
            Self::SubmitBlock(x) => x.is_empty(),
            Self::GetLastBlockHeader(x) => x.is_empty(),
            Self::GetBlockHeaderByHash(x) => x.is_empty(),
            Self::GetBlockHeaderByHeight(x) => x.is_empty(),
            Self::GetBlockHeadersRange(x) => x.is_empty(),
            Self::GetBlock(x) => x.is_empty(),
            Self::GetInfo(x) => x.is_empty(),
            Self::HardForkInfo(x) => x.is_empty(),
            Self::GetOutputHistogram(x) => x.is_empty(),
            Self::GetVersion(x) => x.is_empty(),
            Self::GetFeeEstimate(x) => x.is_empty(),
            Self::GetTransactionPoolBacklog(x) => x.is_empty(),
            Self::GetMinerData(x) => x.is_empty(),
            Self::AddAuxPow(x) => x.is_empty(),
            Self::GetTxIdsLoose(x) => x.is_empty(),
            Self::GenerateBlocks(x) => x.is_empty(),
            Self::GetConnections(x) => x.is_empty(),
            Self::SetBans(x) => x.is_empty(),
            Self::GetBans(x) => x.is_empty(),
            Self::Banned(x) => x.is_empty(),
            Self::FlushTransactionPool(x) => x.is_empty(),
            Self::GetCoinbaseTxSum(x) => x.is_empty(),
            Self::GetAlternateChains(x) => x.is_empty(),
            Self::RelayTx(x) => x.is_empty(),
            Self::SyncInfo(x) => x.is_empty(),
            Self::PruneBlockchain(x) => x.is_empty(),
            Self::CalcPow(x) => x.is_empty(),
            Self::FlushCache(x) => x.is_empty(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Response
/// JSON-RPC responses.
///
/// This enum contains all [`crate::json`] responses.
///
/// See also: [`JsonRpcRequest`].
///
/// # (De)serialization
/// The `serde` implementation will (de)serialize from
/// the inner variant itself, e.g. [`JsonRpcRequest::Banned`]
/// has the same (de)serialization as [`BannedResponse`].
///
/// ```rust
/// use cuprate_rpc_types::{misc::*, json::*};
///
/// let response = JsonRpcResponse::Banned(BannedResponse {
///     banned: true,
///     seconds: 123,
///     status: Status::Ok,
/// });
/// let json = serde_json::to_string(&response).unwrap();
/// assert_eq!(json, r#"{"banned":true,"seconds":123,"status":"OK"}"#);
/// let response: JsonRpcResponse = serde_json::from_str(&json).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(untagged, rename_all = "snake_case"))]
#[allow(missing_docs)]
pub enum JsonRpcResponse {
    GetBlockCount(GetBlockCountResponse),
    OnGetBlockHash(OnGetBlockHashResponse),
    SubmitBlock(SubmitBlockResponse),
    GenerateBlocks(GenerateBlocksResponse),
    GetLastBlockHeader(GetLastBlockHeaderResponse),
    GetBlockHeaderByHash(GetBlockHeaderByHashResponse),
    GetBlockHeaderByHeight(GetBlockHeaderByHeightResponse),
    GetBlockHeadersRange(GetBlockHeadersRangeResponse),
    GetBlock(GetBlockResponse),
    GetConnections(GetConnectionsResponse),
    GetInfo(GetInfoResponse),
    HardForkInfo(HardForkInfoResponse),
    SetBans(SetBansResponse),
    GetBans(GetBansResponse),
    Banned(BannedResponse),
    FlushTransactionPool(FlushTransactionPoolResponse),
    GetOutputHistogram(GetOutputHistogramResponse),
    GetCoinbaseTxSum(GetCoinbaseTxSumResponse),
    GetVersion(GetVersionResponse),
    GetFeeEstimate(GetFeeEstimateResponse),
    GetAlternateChains(GetAlternateChainsResponse),
    RelayTx(RelayTxResponse),
    SyncInfo(SyncInfoResponse),
    GetTransactionPoolBacklog(GetTransactionPoolBacklogResponse),
    GetMinerData(GetMinerDataResponse),
    PruneBlockchain(PruneBlockchainResponse),
    CalcPow(CalcPowResponse),
    FlushCache(FlushCacheResponse),
    AddAuxPow(AddAuxPowResponse),
    GetTxIdsLoose(GetTxIdsLooseResponse),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
