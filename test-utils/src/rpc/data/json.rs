//! JSON data from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.

//---------------------------------------------------------------------------------------------------- Import
use crate::rpc::data::macros::define_request_and_response;

//---------------------------------------------------------------------------------------------------- Struct definitions
// This generates 2 const strings:
//
// - `const GET_BLOCK_TEMPLATE_REQUEST: &str = "..."`
// - `const GET_BLOCK_TEMPLATE_RESPONSE: &str = "..."`
//
// with some interconnected documentation.
define_request_and_response! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint (json).
    //
    // Adding `(json_rpc)` after this will trigger the macro to automatically
    // add a `serde_json` test for the request/response data.
    get_block_template (json_rpc),

    // The base const name: the type of the request/response.
    GET_BLOCK_TEMPLATE: &str,

    // The request data.
    Request =
r#"{
    "jsonrpc": "2.0",
    "id": "0",
    "method": "get_block_template",
    "params": {
      "wallet_address": "44GBHzv6ZyQdJkjqZje6KLZ3xSyN1hBSFAnLP6EAqJtCRVzMzZmeXTC2AHKDS9aEDTRKmo6a6o9r9j86pYfhCWDkKjbtcns",
      "reserve_size": 60
    }
}"#;

    // The response data.
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "blockhashing_blob": "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a00000000e0c20372be23d356347091025c5b5e8f2abf83ab618378565cce2b703491523401",
    "blocktemplate_blob": "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "difficulty": 283305047039,
    "difficulty_top64": 0,
    "expected_reward": 600000000000,
    "height": 3195018,
    "next_seed_hash": "",
    "prev_hash": "9d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a",
    "reserved_offset": 131,
    "seed_hash": "e2aa0b7b55042cd48b02e395d78fa66a29815ccc1584e38db2d1f0e8485cd44f",
    "seed_height": 3194880,
    "status": "OK",
    "untrusted": false,
    "wide_difficulty": "0x41f64bf3ff"
  }
}"#;
}

define_request_and_response! {
    get_block_count (json_rpc),
    GET_BLOCK_COUNT: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_block_count"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "count": 3195019,
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    on_get_block_hash (json_rpc),
    ON_GET_BLOCK_HASH: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "on_get_block_hash",
  "params": [912345]
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6"
}"#;
}

define_request_and_response! {
    submit_block (json_rpc),
    SUBMIT_BLOCK: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "submit_block",
  "params": ["0707e6bdfedc053771512f1bc27c62731ae9e8f2443db64ce742f4e57f5cf8d393de28551e441a0000000002fb830a01ffbf830a018cfe88bee283060274c0aae2ef5730e680308d9c00b6da59187ad0352efe3c71d36eeeb28782f29f2501bd56b952c3ddc3e350c2631d3a5086cac172c56893831228b17de296ff4669de020200000000"]
}"#;
    Response =
r#"{
  "error": {
    "code": -7,
    "message": "Block not accepted"
  },
  "id": "0",
  "jsonrpc": "2.0"
}"#;
}

define_request_and_response! {
    generateblocks (json_rpc),
    GENERATE_BLOCKS: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "generateblocks",
  "params": {
    "amount_of_blocks": 1,
    "wallet_address": "44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A",
    "starting_nonce": 0
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "blocks": ["49b712db7760e3728586f8434ee8bc8d7b3d410dac6bb6e98bf5845c83b917e4"],
    "height": 9783,
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_last_block_header (json_rpc),
    GET_LAST_BLOCK_HEADER: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_last_block_header"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "block_header": {
      "block_size": 200419,
      "block_weight": 200419,
      "cumulative_difficulty": 366125734645190820,
      "cumulative_difficulty_top64": 0,
      "depth": 0,
      "difficulty": 282052561854,
      "difficulty_top64": 0,
      "hash": "57238217820195ac4c08637a144a885491da167899cf1d20e8e7ce0ae0a3434e",
      "height": 3195020,
      "long_term_weight": 200419,
      "major_version": 16,
      "miner_tx_hash": "7a42667237d4f79891bb407c49c712a9299fb87fce799833a7b633a3a9377dbd",
      "minor_version": 16,
      "nonce": 1885649739,
      "num_txes": 37,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "22c72248ae9c5a2863c94735d710a3525c499f70707d1c2f395169bc5c8a0da3",
      "reward": 615702960000,
      "timestamp": 1721245548,
      "wide_cumulative_difficulty": "0x514bd6a74a7d0a4",
      "wide_difficulty": "0x41aba48bbe"
    },
    "credits": 0,
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_block_header_by_hash (json_rpc),
    GET_BLOCK_HEADER_BY_HASH: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_block_header_by_hash",
  "params": {
    "hash": "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6"
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "block_header": {
      "block_size": 210,
      "block_weight": 210,
      "cumulative_difficulty": 754734824984346,
      "cumulative_difficulty_top64": 0,
      "depth": 2282676,
      "difficulty": 815625611,
      "difficulty_top64": 0,
      "hash": "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6",
      "height": 912345,
      "long_term_weight": 210,
      "major_version": 1,
      "miner_tx_hash": "c7da3965f25c19b8eb7dd8db48dcd4e7c885e2491db77e289f0609bf8e08ec30",
      "minor_version": 2,
      "nonce": 1646,
      "num_txes": 0,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "b61c58b2e0be53fad5ef9d9731a55e8a81d972b8d90ed07c04fd37ca6403ff78",
      "reward": 7388968946286,
      "timestamp": 1452793716,
      "wide_cumulative_difficulty": "0x2ae6d65248f1a",
      "wide_difficulty": "0x309d758b"
    },
    "credits": 0,
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_block_header_by_height (json_rpc),
    GET_BLOCK_HEADER_BY_HEIGHT: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_block_header_by_height",
  "params": {
    "height": 912345
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "block_header": {
      "block_size": 210,
      "block_weight": 210,
      "cumulative_difficulty": 754734824984346,
      "cumulative_difficulty_top64": 0,
      "depth": 2282677,
      "difficulty": 815625611,
      "difficulty_top64": 0,
      "hash": "e22cf75f39ae720e8b71b3d120a5ac03f0db50bba6379e2850975b4859190bc6",
      "height": 912345,
      "long_term_weight": 210,
      "major_version": 1,
      "miner_tx_hash": "c7da3965f25c19b8eb7dd8db48dcd4e7c885e2491db77e289f0609bf8e08ec30",
      "minor_version": 2,
      "nonce": 1646,
      "num_txes": 0,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "b61c58b2e0be53fad5ef9d9731a55e8a81d972b8d90ed07c04fd37ca6403ff78",
      "reward": 7388968946286,
      "timestamp": 1452793716,
      "wide_cumulative_difficulty": "0x2ae6d65248f1a",
      "wide_difficulty": "0x309d758b"
    },
    "credits": 0,
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_block_headers_range (json_rpc),
    GET_BLOCK_HEADERS_RANGE: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_block_headers_range",
  "params": {
    "start_height": 1545999,
    "end_height": 1546000
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "headers": [{
      "block_size": 301413,
      "block_weight": 301413,
      "cumulative_difficulty": 13185267971483472,
      "cumulative_difficulty_top64": 0,
      "depth": 1649024,
      "difficulty": 134636057921,
      "difficulty_top64": 0,
      "hash": "86d1d20a40cefcf3dd410ff6967e0491613b77bf73ea8f1bf2e335cf9cf7d57a",
      "height": 1545999,
      "long_term_weight": 301413,
      "major_version": 6,
      "miner_tx_hash": "9909c6f8a5267f043c3b2b079fb4eacc49ef9c1dee1c028eeb1a259b95e6e1d9",
      "minor_version": 6,
      "nonce": 3246403956,
      "num_txes": 20,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "0ef6e948f77b8f8806621003f5de24b1bcbea150bc0e376835aea099674a5db5",
      "reward": 5025593029981,
      "timestamp": 1523002893,
      "wide_cumulative_difficulty": "0x2ed7ee6db56750",
      "wide_difficulty": "0x1f58ef3541"
    },{
      "block_size": 13322,
      "block_weight": 13322,
      "cumulative_difficulty": 13185402687569710,
      "cumulative_difficulty_top64": 0,
      "depth": 1649023,
      "difficulty": 134716086238,
      "difficulty_top64": 0,
      "hash": "b408bf4cfcd7de13e7e370c84b8314c85b24f0ba4093ca1d6eeb30b35e34e91a",
      "height": 1546000,
      "long_term_weight": 13322,
      "major_version": 7,
      "miner_tx_hash": "7f749c7c64acb35ef427c7454c45e6688781fbead9bbf222cb12ad1a96a4e8f6",
      "minor_version": 7,
      "nonce": 3737164176,
      "num_txes": 1,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "86d1d20a40cefcf3dd410ff6967e0491613b77bf73ea8f1bf2e335cf9cf7d57a",
      "reward": 4851952181070,
      "timestamp": 1523002931,
      "wide_cumulative_difficulty": "0x2ed80dcb69bf2e",
      "wide_difficulty": "0x1f5db457de"
    }],
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_block (json_rpc),
    GET_BLOCK: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_block",
  "params": {
    "height": 2751506
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "blob": "1010c58bab9b06b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7807e07f502cef8a70101ff92f8a7010180e0a596bb1103d7cbf826b665d7a532c316982dc8dbc24f285cbc18bbcc27c7164cd9b3277a85d034019f629d8b36bd16a2bfce3ea80c31dc4d8762c67165aec21845494e32b7582fe00211000000297a787a000000000000000000000000",
    "block_header": {
      "block_size": 106,
      "block_weight": 106,
      "cumulative_difficulty": 236046001376524168,
      "cumulative_difficulty_top64": 0,
      "depth": 443517,
      "difficulty": 313732272488,
      "difficulty_top64": 0,
      "hash": "43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428",
      "height": 2751506,
      "long_term_weight": 176470,
      "major_version": 16,
      "miner_tx_hash": "e49b854c5f339d7410a77f2a137281d8042a0ffc7ef9ab24cd670b67139b24cd",
      "minor_version": 16,
      "nonce": 4110909056,
      "num_txes": 0,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7",
      "reward": 600000000000,
      "timestamp": 1667941829,
      "wide_cumulative_difficulty": "0x3469a966eb2f788",
      "wide_difficulty": "0x490be69168"
    },
    "credits": 0,
    "json": "{\n  \"major_version\": 16, \n  \"minor_version\": 16, \n  \"timestamp\": 1667941829, \n  \"prev_id\": \"b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7\", \n  \"nonce\": 4110909056, \n  \"miner_tx\": {\n    \"version\": 2, \n    \"unlock_time\": 2751566, \n    \"vin\": [ {\n        \"gen\": {\n          \"height\": 2751506\n        }\n      }\n    ], \n    \"vout\": [ {\n        \"amount\": 600000000000, \n        \"target\": {\n          \"tagged_key\": {\n            \"key\": \"d7cbf826b665d7a532c316982dc8dbc24f285cbc18bbcc27c7164cd9b3277a85\", \n            \"view_tag\": \"d0\"\n          }\n        }\n      }\n    ], \n    \"extra\": [ 1, 159, 98, 157, 139, 54, 189, 22, 162, 191, 206, 62, 168, 12, 49, 220, 77, 135, 98, 198, 113, 101, 174, 194, 24, 69, 73, 78, 50, 183, 88, 47, 224, 2, 17, 0, 0, 0, 41, 122, 120, 122, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0\n    ], \n    \"rct_signatures\": {\n      \"type\": 0\n    }\n  }, \n  \"tx_hashes\": [ ]\n}",
    "miner_tx_hash": "e49b854c5f339d7410a77f2a137281d8042a0ffc7ef9ab24cd670b67139b24cd",
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_block (json_rpc),
    /// This is the same as [`GET_BLOCK_REQUEST`] and
    /// [`GET_BLOCK_RESPONSE`] but it uses the `hash` parameter.
    GET_BLOCK_HASH: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_block",
  "params": {
    "hash": "86d421322b700166dde2d7eba1cc8600925ef640abf6c0a2cc8ce0d6dd90abfd"
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "blob": "1010d8faa89b06f8a36d0dbe4d27d2f52160000563896048d71067c31e99a3869bf9b7142227bb5328010b02a6f6a70101ffeaf5a70101a08bc8b3bb11036d6713f5aa552a1aaf33baed7591f795b86daf339e51029a9062dfe09f0f909b312b0124d6023d591c4d434000e5e31c6db718a1e96e865939930e90a7042a1cd4cbd202083786a78452fdfc000002a89e380a44d8dfc64b551baa171447a0f9c9262255be6e8f8ef10896e36e2bf90c4d343e416e394ad9cc10b7d2df7b2f39370a554730f75dfcb04944bd62c299",
    "block_header": {
      "block_size": 3166,
      "block_weight": 3166,
      "cumulative_difficulty": 235954020187853162,
      "cumulative_difficulty_top64": 0,
      "depth": 443814,
      "difficulty": 312527777859,
      "difficulty_top64": 0,
      "hash": "86d421322b700166dde2d7eba1cc8600925ef640abf6c0a2cc8ce0d6dd90abfd",
      "height": 2751210,
      "long_term_weight": 176470,
      "major_version": 16,
      "miner_tx_hash": "dabe07900d3123ed895612f4a151adb3e39681b145f0f85bfee23ea1fe47acf2",
      "minor_version": 16,
      "nonce": 184625235,
      "num_txes": 2,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "f8a36d0dbe4d27d2f52160000563896048d71067c31e99a3869bf9b7142227bb",
      "reward": 600061380000,
      "timestamp": 1667906904,
      "wide_cumulative_difficulty": "0x34646ee649f516a",
      "wide_difficulty": "0x48c41b7043"
    },
    "credits": 0,
    "json": "{\n  \"major_version\": 16, \n  \"minor_version\": 16, \n  \"timestamp\": 1667906904, \n  \"prev_id\": \"f8a36d0dbe4d27d2f52160000563896048d71067c31e99a3869bf9b7142227bb\", \n  \"nonce\": 184625235, \n  \"miner_tx\": {\n    \"version\": 2, \n    \"unlock_time\": 2751270, \n    \"vin\": [ {\n        \"gen\": {\n          \"height\": 2751210\n        }\n      }\n    ], \n    \"vout\": [ {\n        \"amount\": 600061380000, \n        \"target\": {\n          \"tagged_key\": {\n            \"key\": \"6d6713f5aa552a1aaf33baed7591f795b86daf339e51029a9062dfe09f0f909b\", \n            \"view_tag\": \"31\"\n          }\n        }\n      }\n    ], \n    \"extra\": [ 1, 36, 214, 2, 61, 89, 28, 77, 67, 64, 0, 229, 227, 28, 109, 183, 24, 161, 233, 110, 134, 89, 57, 147, 14, 144, 167, 4, 42, 28, 212, 203, 210, 2, 8, 55, 134, 167, 132, 82, 253, 252, 0\n    ], \n    \"rct_signatures\": {\n      \"type\": 0\n    }\n  }, \n  \"tx_hashes\": [ \"a89e380a44d8dfc64b551baa171447a0f9c9262255be6e8f8ef10896e36e2bf9\", \"0c4d343e416e394ad9cc10b7d2df7b2f39370a554730f75dfcb04944bd62c299\"\n  ]\n}",
    "miner_tx_hash": "dabe07900d3123ed895612f4a151adb3e39681b145f0f85bfee23ea1fe47acf2",
    "status": "OK",
    "top_hash": "",
    "tx_hashes": ["a89e380a44d8dfc64b551baa171447a0f9c9262255be6e8f8ef10896e36e2bf9","0c4d343e416e394ad9cc10b7d2df7b2f39370a554730f75dfcb04944bd62c299"],
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_connections (json_rpc),
    GET_CONNECTIONS: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_connections"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "connections": [{
      "address": "3evk3kezfjg44ma6tvesy7rbxwwpgpympj45xar5fo4qajrsmkoaqdqd.onion:18083",
      "address_type": 4,
      "avg_download": 0,
      "avg_upload": 0,
      "connection_id": "22ef856d0f1d44cc95e84fecfd065fe2",
      "current_download": 0,
      "current_upload": 0,
      "height": 3195026,
      "host": "3evk3kezfjg44ma6tvesy7rbxwwpgpympj45xar5fo4qajrsmkoaqdqd.onion",
      "incoming": false,
      "ip": "",
      "live_time": 76651,
      "local_ip": false,
      "localhost": false,
      "peer_id": "0000000000000001",
      "port": "",
      "pruning_seed": 0,
      "recv_count": 240328,
      "recv_idle_time": 34,
      "rpc_credits_per_hash": 0,
      "rpc_port": 0,
      "send_count": 3406572,
      "send_idle_time": 30,
      "state": "normal",
      "support_flags": 0
    },{
      "address": "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion:18083",
      "address_type": 4,
      "avg_download": 0,
      "avg_upload": 0,
      "connection_id": "c7734e15936f485a86d2b0534f87e499",
      "current_download": 0,
      "current_upload": 0,
      "height": 3195024,
      "host": "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion",
      "incoming": false,
      "ip": "",
      "live_time": 76755,
      "local_ip": false,
      "localhost": false,
      "peer_id": "0000000000000001",
      "port": "",
      "pruning_seed": 389,
      "recv_count": 237657,
      "recv_idle_time": 120,
      "rpc_credits_per_hash": 0,
      "rpc_port": 0,
      "send_count": 3370566,
      "send_idle_time": 120,
      "state": "normal",
      "support_flags": 0
    }],
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_info (json_rpc),
    GET_INFO: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_info"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "adjusted_time": 1721245289,
    "alt_blocks_count": 16,
    "block_size_limit": 600000,
    "block_size_median": 300000,
    "block_weight_limit": 600000,
    "block_weight_median": 300000,
    "bootstrap_daemon_address": "",
    "busy_syncing": false,
    "credits": 0,
    "cumulative_difficulty": 366127702242611947,
    "cumulative_difficulty_top64": 0,
    "database_size": 235169075200,
    "difficulty": 280716748706,
    "difficulty_top64": 0,
    "free_space": 30521749504,
    "grey_peerlist_size": 4996,
    "height": 3195028,
    "height_without_bootstrap": 3195028,
    "incoming_connections_count": 62,
    "mainnet": true,
    "nettype": "mainnet",
    "offline": false,
    "outgoing_connections_count": 1143,
    "restricted": false,
    "rpc_connections_count": 1,
    "stagenet": false,
    "start_time": 1720462427,
    "status": "OK",
    "synchronized": true,
    "target": 120,
    "target_height": 0,
    "testnet": false,
    "top_block_hash": "bdf06d18ed1931a8ee62654e9b6478cc459bc7072628b8e36f4524d339552946",
    "top_hash": "",
    "tx_count": 43205750,
    "tx_pool_size": 12,
    "untrusted": false,
    "update_available": false,
    "version": "0.18.3.3-release",
    "was_bootstrap_ever_used": false,
    "white_peerlist_size": 1000,
    "wide_cumulative_difficulty": "0x514bf349299d2eb",
    "wide_difficulty": "0x415c05a7a2"
  }
}"#;
}

define_request_and_response! {
    hard_fork_info (json_rpc),
    HARD_FORK_INFO: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "hard_fork_info",
  "params": {
    "version": 16
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "earliest_height": 2689608,
    "enabled": true,
    "state": 0,
    "status": "OK",
    "threshold": 0,
    "top_hash": "",
    "untrusted": false,
    "version": 16,
    "votes": 10080,
    "voting": 16,
    "window": 10080
  }
}"#;
}

define_request_and_response! {
    set_bans (json_rpc),
    SET_BANS: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "set_bans",
  "params": {
    "bans": [{
      "host": "192.168.1.51",
      "ban": true,
      "seconds": 30
    }]
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    set_bans (json_rpc),
    /// This is the same as [`SET_BANS_REQUEST`] and
    /// [`SET_BANS_RESPONSE`] but it uses the `ip` parameter.
    SET_BANS_IP: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "set_bans",
  "params": {
    "bans": [{
      "ip": 838969536,
      "ban": true,
      "seconds": 30
    }]
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_bans (json_rpc),
    GET_BANS: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_bans"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "bans": [{
      "host": "104.248.206.131",
      "ip": 2211379304,
      "seconds": 689754
    },{
      "host": "209.222.252.0\/24",
      "ip": 0,
      "seconds": 689754
    }],
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    banned (json_rpc),
    BANNED: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "banned",
  "params": {
    "address": "95.216.203.255"
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "banned": true,
    "seconds": 689655,
    "status": "OK"
  }
}"#;
}

define_request_and_response! {
    flush_txpool (json_rpc),
    FLUSH_TRANSACTION_POOL: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "flush_txpool",
  "params": {
    "txids": ["dc16fa8eaffe1484ca9014ea050e13131d3acf23b419f33bb4cc0b32b6c49308"]
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "status": "OK"
  }
}"#;
}

define_request_and_response! {
    get_output_histogram (json_rpc),
    GET_OUTPUT_HISTOGRAM: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_output_histogram",
  "params": {
    "amounts": [20000000000]
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "histogram": [{
      "amount": 20000000000,
      "recent_instances": 0,
      "total_instances": 381490,
      "unlocked_instances": 0
    }],
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_coinbase_tx_sum (json_rpc),
    GET_COINBASE_TX_SUM: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_coinbase_tx_sum",
  "params": {
    "height": 1563078,
    "count": 2
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "emission_amount": 9387854817320,
    "emission_amount_top64": 0,
    "fee_amount": 83981380000,
    "fee_amount_top64": 0,
    "status": "OK",
    "top_hash": "",
    "untrusted": false,
    "wide_emission_amount": "0x889c7c06828",
    "wide_fee_amount": "0x138dae29a0"
  }
}"#;
}

define_request_and_response! {
    get_version (json_rpc),
    GET_VERSION: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_version"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "current_height": 3195051,
    "hard_forks": [{
      "height": 1,
      "hf_version": 1
    },{
      "height": 1009827,
      "hf_version": 2
    },{
      "height": 1141317,
      "hf_version": 3
    },{
      "height": 1220516,
      "hf_version": 4
    },{
      "height": 1288616,
      "hf_version": 5
    },{
      "height": 1400000,
      "hf_version": 6
    },{
      "height": 1546000,
      "hf_version": 7
    },{
      "height": 1685555,
      "hf_version": 8
    },{
      "height": 1686275,
      "hf_version": 9
    },{
      "height": 1788000,
      "hf_version": 10
    },{
      "height": 1788720,
      "hf_version": 11
    },{
      "height": 1978433,
      "hf_version": 12
    },{
      "height": 2210000,
      "hf_version": 13
    },{
      "height": 2210720,
      "hf_version": 14
    },{
      "height": 2688888,
      "hf_version": 15
    },{
      "height": 2689608,
      "hf_version": 16
    }],
    "release": true,
    "status": "OK",
    "untrusted": false,
    "version": 196621
  }
}"#;
}

define_request_and_response! {
    get_fee_estimate (json_rpc),
    GET_FEE_ESTIMATE: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_fee_estimate"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "fee": 20000,
    "fees": [20000,80000,320000,4000000],
    "quantization_mask": 10000,
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_alternate_chains (json_rpc),
    GET_ALTERNATE_CHAINS: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_alternate_chains"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "chains": [{
      "block_hash": "4826c7d45d7cf4f02985b5c405b0e5d7f92c8d25e015492ce19aa3b209295dce",
      "block_hashes": ["4826c7d45d7cf4f02985b5c405b0e5d7f92c8d25e015492ce19aa3b209295dce"],
      "difficulty": 357404825113208373,
      "difficulty_top64": 0,
      "height": 3167471,
      "length": 1,
      "main_chain_parent_block": "69b5075ea627d6ba06b1c30b7e023884eeaef5282cf58ec847dab838ddbcdd86",
      "wide_difficulty": "0x4f5c1cb79e22635"
    },{
      "block_hash": "33ee476f5a1c5b9d889274cbbe171f5e0112df7ed69021918042525485deb401",
      "block_hashes": ["33ee476f5a1c5b9d889274cbbe171f5e0112df7ed69021918042525485deb401"],
      "difficulty": 354736121711617293,
      "difficulty_top64": 0,
      "height": 3157465,
      "length": 1,
      "main_chain_parent_block": "fd522fcc4cefe5c8c0e5c5600981b3151772c285df3a4e38e5c4011cf466d2cb",
      "wide_difficulty": "0x4ec469f8b9ee50d"
    }],
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    relay_tx (json_rpc),
    RELAY_TX: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "relay_tx",
  "params": {
    "txids": ["9fd75c429cbe52da9a52f2ffc5fbd107fe7fd2099c0d8de274dc8a67e0c98613"]
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "status": "OK"
  }
}"#;
}

define_request_and_response! {
    sync_info (json_rpc),
    SYNC_INFO: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "sync_info"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "height": 3195157,
    "next_needed_pruning_seed": 0,
    "overview": "[]",
    "peers": [{
      "info": {
        "address": "142.93.128.65:44986",
        "address_type": 1,
        "avg_download": 1,
        "avg_upload": 1,
        "connection_id": "a5803c4c2dac49e7b201dccdef54c862",
        "current_download": 2,
        "current_upload": 1,
        "height": 3195157,
        "host": "142.93.128.65",
        "incoming": true,
        "ip": "142.93.128.65",
        "live_time": 18,
        "local_ip": false,
        "localhost": false,
        "peer_id": "6830e9764d3e5687",
        "port": "44986",
        "pruning_seed": 0,
        "recv_count": 20340,
        "recv_idle_time": 0,
        "rpc_credits_per_hash": 0,
        "rpc_port": 18089,
        "send_count": 32235,
        "send_idle_time": 6,
        "state": "normal",
        "support_flags": 1
      }
    },{
      "info": {
        "address": "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion:18083",
        "address_type": 4,
        "avg_download": 0,
        "avg_upload": 0,
        "connection_id": "277f7c821bc546878c8bd29977e780f5",
        "current_download": 0,
        "current_upload": 0,
        "height": 3195157,
        "host": "4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion",
        "incoming": false,
        "ip": "",
        "live_time": 2246,
        "local_ip": false,
        "localhost": false,
        "peer_id": "0000000000000001",
        "port": "",
        "pruning_seed": 389,
        "recv_count": 65164,
        "recv_idle_time": 15,
        "rpc_credits_per_hash": 0,
        "rpc_port": 0,
        "send_count": 99120,
        "send_idle_time": 15,
        "state": "normal",
        "support_flags": 0
      }
    }],
    "status": "OK",
    "target_height": 0,
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_txpool_backlog_v2 (json_rpc),
    GET_TRANSACTION_POOL_BACKLOG_V2: &str,
    Request =
r#"{
   "jsonrpc": "2.0",
   "id": "0",
   "method": "get_txpool_backlog_v2"
 }"#;
    Response =
r#"{
   "id": "0",
   "jsonrpc": "2.0",
   "result": {
     "backlog": [
        {
          weight: 0,
          fee: 0,
          time_in_pool: 0,
        }
     ],
     "status": "OK",
     "untrusted": false
   }
 }"#;
}

define_request_and_response! {
    get_output_distribution_v2 (json_rpc),
    GET_OUTPUT_DISTRIBUTION_V2: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_output_distribution",
  "params": {
    "amounts": [628780000],
    "from_height": 1462078
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "credits": 0,
    "distributions": [{
      "amount": 2628780000,
      "base": 0,
      "distribution": "",
      "start_height": 1462078,
      "binary": false
    }],
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    get_miner_data (json_rpc),
    GET_MINER_DATA: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_miner_data"
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "already_generated_coins": 18186022843595960691,
    "difficulty": "0x48afae42de",
    "height": 2731375,
    "major_version": 16,
    "median_weight": 300000,
    "prev_id": "78d50c5894d187c4946d54410990ca59a75017628174a9e8c7055fa4ca5c7c6d",
    "seed_hash": "a6b869d50eca3a43ec26fe4c369859cf36ae37ce6ecb76457d31ffeb8a6ca8a6",
    "status": "OK",
    "tx_backlog": [{
      "fee": 30700000,
      "id": "9868490d6bb9207fdd9cf17ca1f6c791b92ca97de0365855ea5c089f67c22208",
      "weight": 1535
    },{
      "fee": 44280000,
      "id": "b6000b02bbec71e18ad704bcae09fb6e5ae86d897ced14a718753e76e86c0a0a",
      "weight": 2214
    }],
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    prune_blockchain (json_rpc),
    PRUNE_BLOCKCHAIN: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "prune_blockchain",
  "params": {
    "check": true
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "pruned": true,
    "pruning_seed": 387,
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    calc_pow (json_rpc),
    CALC_POW: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "calc_pow",
  "params": {
    "major_version": 14,
    "height": 2286447,
    "block_blob": "0e0ed286da8006ecdc1aab3033cf1716c52f13f9d8ae0051615a2453643de94643b550d543becd0000000002abc78b0101ffefc68b0101fcfcf0d4b422025014bb4a1eade6622fd781cb1063381cad396efa69719b41aa28b4fce8c7ad4b5f019ce1dc670456b24a5e03c2d9058a2df10fec779e2579753b1847b74ee644f16b023c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000051399a1bc46a846474f5b33db24eae173a26393b976054ee14f9feefe99925233802867097564c9db7a36af5bb5ed33ab46e63092bd8d32cef121608c3258edd55562812e21cc7e3ac73045745a72f7d74581d9a0849d6f30e8b2923171253e864f4e9ddea3acb5bc755f1c4a878130a70c26297540bc0b7a57affb6b35c1f03d8dbd54ece8457531f8cba15bb74516779c01193e212050423020e45aa2c15dcb",
    "seed_hash": "d432f499205150873b2572b5f033c9c6e4b7c6f3394bd2dd93822cd7085e7307"
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": "d0402d6834e26fb94a9ce38c6424d27d2069896a9b8b1ce685d79936bca6e0a8"
}"#;
}

define_request_and_response! {
    flush_cache (json_rpc),
    FLUSH_CACHE: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "flush_cache",
  "params": {
    "bad_txs": true,
    "bad_blocks": true
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    add_aux_pow (json_rpc),
    ADD_AUX_POW: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "add_aux_pow",
  "params": {
    "blocktemplate_blob": "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "aux_pow": [{
      "id": "3200b4ea97c3b2081cd4190b58e49572b2319fed00d030ad51809dff06b5d8c8",
      "hash": "7b35762de164b20885e15dbe656b1138db06bb402fa1796f5765a23933d8859a"
    }]
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "aux_pow": [{
      "hash": "7b35762de164b20885e15dbe656b1138db06bb402fa1796f5765a23933d8859a",
      "id": "3200b4ea97c3b2081cd4190b58e49572b2319fed00d030ad51809dff06b5d8c8"
    }],
    "blockhashing_blob": "1010ee97e2a106e9f8ebe8887e5b609949ac8ea6143e560ed13552b110cb009b21f0cfca1eaccf00000000b2685c1283a646bc9020c758daa443be145b7370ce5a6efacb3e614117032e2c22",
    "blocktemplate_blob": "1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "merkle_root": "7b35762de164b20885e15dbe656b1138db06bb402fa1796f5765a23933d8859a",
    "merkle_tree_depth": 0,
    "status": "OK",
    "untrusted": false
  }
}"#;
}

define_request_and_response! {
    UNDOCUMENTED_METHOD (json_rpc),
    GET_TX_IDS_LOOSE: &str,
    Request =
r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "get_txids_loose",
  "params": {
    "txid_template": "0000000000000000aea473c43708aa50b2c9eaf0e441aa209afc9b43458fb09e",
    "num_matching_bits": 192
  }
}"#;
    Response =
r#"{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "txids": "",
    "status": "OK",
    "untrusted": false
  }
}"#;
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
