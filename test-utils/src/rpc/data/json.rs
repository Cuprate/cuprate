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
    // Adding `(json)` after this will trigger the macro to automatically
    // add a `serde_json` test for the request/response data.
    get_block_template (json),

    // The base const name: the type of the request/response.
    GET_BLOCK_TEMPLATE: &str,

    // The request literal.
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

    // The response literal.
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
    get_block_count (json),
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
    on_get_block_hash (json),
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
    submit_block (json),
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
    generateblocks (json),
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
    get_last_block_header (json),
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
    get_block_header_by_hash (json),
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
    get_block_header_by_height (json),
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
    get_block_headers_range (json),
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
    get_block (json),
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
    get_block (json),
    /// This is the same as [`GET_BLOCK_REQUEST`] and
    /// [`GET_BLOCK_RESPONSE`] but it uses the `hash` parameter.
    GET_BLOCK_BY_HASH: &str,
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
    get_connections (json),
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
    get_info (json),
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

// define_request_and_response! {
//     hard_fork_info (json),
//     HardForkInfo,
//     Request = r#""#;
//     Response = r#""#;
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
//     set_bans (json),
//     SetBans,
//     Request = r#""#;
//         bans: Vec<SetBan>,
//     },
//     Response =
// r#""#;
// }

// define_request_and_response! {
//     get_bans (json),
//     GetBans,
//     Request = r#""#;
//     Response =
// r#""#;
//         bans: Vec<GetBan>,
//     }
// }

// define_request_and_response! {
//     banned (json),
//     Banned,
//     #[cfg_attr(feature = "serde", serde(transparent))]
//     #[repr(transparent)]
//     Request = r#""#;
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
//     flush_txpool (json),
//     FlushTransactionPool,
//     Request = r#""#;
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
//     get_output_histogram (json),
//     GetOutputHistogram,
//     Request = r#""#;
//         amounts: Vec<u64>,
//         min_count: u64,
//         max_count: u64,
//         unlocked: bool,
//         recent_cutoff: u64,
//     },
//     Response = r#""#;
//         histogram: Vec<HistogramEntry>,
//     }
// }

// define_request_and_response! {
//     get_coinbase_tx_sum (json),
//     GetCoinbaseTxSum,
//     Request = r#""#;
//         height: u64,
//         count: u64,
//     },
//     Response = r#""#;
//         emission_amount: u64,
//         emission_amount_top64: u64,
//         fee_amount: u64,
//         fee_amount_top64: u64,
//         wide_emission_amount: String,
//         wide_fee_amount: String,
//     }
// }

// define_request_and_response! {
//     get_version (json),
//     GetVersion,
//     Request = r#""#;
//     Response =
// r#""#;
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
//     get_fee_estimate (json),
//     GetFeeEstimate,
//     Request = r#""#;
//     Response = r#""#;
//         fee: u64,
//         fees: Vec<u64>,
//         #[serde(skip_serializing_if = "is_one")]
//         quantization_mask: u64,
//     }
// }

// define_request_and_response! {
//     get_alternate_chains (json),
//     GetAlternateChains,
//     Request = r#""#;
//     Response =
// r#""#;
//         chains: Vec<ChainInfo>,
//     }
// }

// define_request_and_response! {
//     relay_tx (json),
//     RelayTx,
//     Request = r#""#;
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
//     sync_info (json),
//     SyncInfo,
//     Request = r#""#;
//     Response = r#""#;
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
//     get_txpool_backlog (json),
//     GetTransactionPoolBacklog,
//     Request = r#""#;
//     Response =
// r#""#;
//         // TODO: this is a [`BinaryString`].
//         backlog: Vec<TxBacklogEntry>,
//     }
// }

// define_request_and_response! {
//     get_output_distribution (json),
//     /// This type is also used in the (undocumented)
//     GetOutputDistribution,
//     Request = r#""#;
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
//     get_miner_data (json),
//     GetMinerData,
//     Request = r#""#;
//     Response =
// r#""#;
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
//     prune_blockchain (json),
//     PruneBlockchain,
//     #[derive(Copy)]
//     Request = r#""#;
//         check: bool = default_false(), "default_false",
//     },
//     #[derive(Copy)]
//     Response =
// r#""#;
//         pruned: bool,
//         pruning_seed: u32,
//     }
// }

// define_request_and_response! {
//     calc_pow (json),
//     CalcPow,
//     Request = r#""#;
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
//     flush_cache (json),
//     FlushCache,
//     #[derive(Copy)]
//     Request = r#""#;
//         bad_txs: bool = default_false(), "default_false",
//         bad_blocks: bool = default_false(), "default_false",
//     },
//     Response =
// r#""#;
// }

// define_request_and_response! {
//     add_aux_pow (json),
//     AddAuxPow,
//     Request = r#""#;
//         blocktemplate_blob: String,
//         aux_pow: Vec<AuxPow>,
//     },
//     Response =
// r#""#;
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
