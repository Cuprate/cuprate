//! Tests and utilities.

#![allow(
    clippy::unreadable_literal,
    clippy::manual_string_new,
    clippy::struct_field_names
)]

//---------------------------------------------------------------------------------------------------- Use
use std::borrow::Cow;

use pretty_assertions::assert_eq;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{to_string, to_string_pretty, to_value, Value};

use crate::{Id, Request, Response};

//---------------------------------------------------------------------------------------------------- Body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct Body<P> {
    pub(crate) method: Cow<'static, str>,
    pub(crate) params: P,
}

//---------------------------------------------------------------------------------------------------- Free functions
/// Assert input and output of serialization are the same.
pub(crate) fn assert_ser<T>(t: &T, expected_value: &Value)
where
    T: Serialize + std::fmt::Debug + Clone + PartialEq,
{
    let value = to_value(t).unwrap();
    assert_eq!(value, *expected_value);
}

/// Assert input and output of string serialization are the same.
pub(crate) fn assert_ser_string<T>(t: &T, expected_string: &str)
where
    T: Serialize + std::fmt::Debug + Clone + PartialEq,
{
    let string = to_string(t).unwrap();
    assert_eq!(string, expected_string);
}

/// Assert input and output of (pretty) string serialization are the same.
pub(crate) fn assert_ser_string_pretty<T>(t: &T, expected_string: &str)
where
    T: Serialize + std::fmt::Debug + Clone + PartialEq,
{
    let string = to_string_pretty(t).unwrap();
    assert_eq!(string, expected_string);
}

/// Tests an input JSON string matches an expected type `T`.
fn assert_de<T>(json: &'static str, expected: T)
where
    T: DeserializeOwned + std::fmt::Debug + Clone + PartialEq,
{
    let t = serde_json::from_str::<T>(json).unwrap();
    assert_eq!(t, expected);
}

//---------------------------------------------------------------------------------------------------- Types
// Parameter type.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct GetBlock {
    height: u64,
}

// Method enum containing all params.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", content = "params")]
#[serde(rename_all = "snake_case")]
enum Methods {
    GetBlock(GetBlock),
    GetBlockCount,
}

//---------------------------------------------------------------------------------------------------- TESTS
/// Tests that Monero's `get_block` request and response
/// in JSON string form gets correctly (de)serialized.
#[test]
fn monero_jsonrpc_get_block() {
    //--- Request
    const REQUEST: &str =
        r#"{"jsonrpc":"2.0","id":"0","method":"get_block","params":{"height":123}}"#;
    let request = Request::new_with_id(
        Id::Str("0".into()),
        Methods::GetBlock(GetBlock { height: 123 }),
    );
    assert_ser_string(&request, REQUEST);
    assert_de(REQUEST, request);

    //--- Response
    const RESPONSE: &str = r#"{
  "jsonrpc": "2.0",
  "id": "0",
  "result": {
    "blob": "01008cb1c49a0572244e0c8b2b8b99236e10c03eba53685b346aab525eb20b59a459b5935cd5a5aaa8f2ba01b70101ff7b08ebcc2202f917ac2dc38c0e0735f2c97df4a307a445b32abaf0ad528c385ae11a7e767d3880897a0215a39af4cf4c67136ecc048d9296b4cb7a6be61275a0ef207eb4cbb427cc216380dac40902dabddeaada9f4ed2512f9b9613a7ced79d3996ad5050ca542f31032bd638193380c2d72f02965651ab4a26264253bb8a4ccb9b33afbc8c8b4f3e331baf50537b8ee80364038088aca3cf020235a7367536243629560b8a40f104352c89a2d4719e86f54175c2e4e3ecfec9938090cad2c60e023b623a01eace71e9b37d2bfac84f9aafc85dbf62a0f452446c5de0ca50cf910580e08d84ddcb010258c370ee02069943e5440294aeafae29656f9782b0a565d26065bb7af07a6af980c0caf384a30202899a53eeb05852a912bcbc6fa78e4c85f0b059726b0b8f0753e7aa54fc9d7ce82101351af203765d1679e2a9458ab6737d289e18c49766d41fc31a2bf0fe32dd196200",
    "block_header": {
      "block_size": 345,
      "block_weight": 345,
      "cumulative_difficulty": 4646953,
      "cumulative_difficulty_top64": 0,
      "depth": 3166236,
      "difficulty": 51263,
      "difficulty_top64": 0,
      "hash": "ff617b489e91f5db76f6f2cc9b3f03236e09fb191b4238f4a1e64185a6c28019",
      "height": 123,
      "long_term_weight": 345,
      "major_version": 1,
      "miner_tx_hash": "054ba33024e72dfe8dafabc8af7c0e070e9aca3a9df44569cfa3c96669f9542f",
      "minor_version": 0,
      "nonce": 3136465066,
      "num_txes": 0,
      "orphan_status": false,
      "pow_hash": "",
      "prev_hash": "72244e0c8b2b8b99236e10c03eba53685b346aab525eb20b59a459b5935cd5a5",
      "reward": 17590122566891,
      "timestamp": 1397823628,
      "wide_cumulative_difficulty": "0x46e829",
      "wide_difficulty": "0xc83f"
    },
    "credits": 0,
    "json": "{\n  \"major_version\": 1, \n  \"minor_version\": 0, \n  \"timestamp\": 1397823628, \n  \"prev_id\": \"72244e0c8b2b8b99236e10c03eba53685b346aab525eb20b59a459b5935cd5a5\", \n  \"nonce\": 3136465066, \n  \"miner_tx\": {\n    \"version\": 1, \n    \"unlock_time\": 183, \n    \"vin\": [ {\n        \"gen\": {\n          \"height\": 123\n        }\n      }\n    ], \n    \"vout\": [ {\n        \"amount\": 566891, \n        \"target\": {\n          \"key\": \"f917ac2dc38c0e0735f2c97df4a307a445b32abaf0ad528c385ae11a7e767d38\"\n        }\n      }, {\n        \"amount\": 2000000, \n        \"target\": {\n          \"key\": \"15a39af4cf4c67136ecc048d9296b4cb7a6be61275a0ef207eb4cbb427cc2163\"\n        }\n      }, {\n        \"amount\": 20000000, \n        \"target\": {\n          \"key\": \"dabddeaada9f4ed2512f9b9613a7ced79d3996ad5050ca542f31032bd6381933\"\n        }\n      }, {\n        \"amount\": 100000000, \n        \"target\": {\n          \"key\": \"965651ab4a26264253bb8a4ccb9b33afbc8c8b4f3e331baf50537b8ee8036403\"\n        }\n      }, {\n        \"amount\": 90000000000, \n        \"target\": {\n          \"key\": \"35a7367536243629560b8a40f104352c89a2d4719e86f54175c2e4e3ecfec993\"\n        }\n      }, {\n        \"amount\": 500000000000, \n        \"target\": {\n          \"key\": \"3b623a01eace71e9b37d2bfac84f9aafc85dbf62a0f452446c5de0ca50cf9105\"\n        }\n      }, {\n        \"amount\": 7000000000000, \n        \"target\": {\n          \"key\": \"58c370ee02069943e5440294aeafae29656f9782b0a565d26065bb7af07a6af9\"\n        }\n      }, {\n        \"amount\": 10000000000000, \n        \"target\": {\n          \"key\": \"899a53eeb05852a912bcbc6fa78e4c85f0b059726b0b8f0753e7aa54fc9d7ce8\"\n        }\n      }\n    ], \n    \"extra\": [ 1, 53, 26, 242, 3, 118, 93, 22, 121, 226, 169, 69, 138, 182, 115, 125, 40, 158, 24, 196, 151, 102, 212, 31, 195, 26, 43, 240, 254, 50, 221, 25, 98\n    ], \n    \"signatures\": [ ]\n  }, \n  \"tx_hashes\": [ ]\n}",
    "miner_tx_hash": "054ba33024e72dfe8dafabc8af7c0e070e9aca3a9df44569cfa3c96669f9542f",
    "status": "OK",
    "top_hash": "",
    "untrusted": false
  }
}"#;

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct Json {
        blob: String,
        block_header: BlockHeader,
        credits: u64,
        json: String,
        miner_tx_hash: String,
        status: String,
        top_hash: String,
        untrusted: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct BlockHeader {
        block_size: u64,
        block_weight: u64,
        cumulative_difficulty: u64,
        cumulative_difficulty_top64: u64,
        depth: u64,
        difficulty: u64,
        difficulty_top64: u64,
        hash: String,
        height: u64,
        long_term_weight: u64,
        major_version: u64,
        miner_tx_hash: String,
        minor_version: u64,
        nonce: u64,
        num_txes: u64,
        orphan_status: bool,
        pow_hash: String,
        prev_hash: String,
        reward: u64,
        timestamp: u64,
        wide_cumulative_difficulty: String,
        wide_difficulty: String,
    }

    let payload = Json {
        blob: "01008cb1c49a0572244e0c8b2b8b99236e10c03eba53685b346aab525eb20b59a459b5935cd5a5aaa8f2ba01b70101ff7b08ebcc2202f917ac2dc38c0e0735f2c97df4a307a445b32abaf0ad528c385ae11a7e767d3880897a0215a39af4cf4c67136ecc048d9296b4cb7a6be61275a0ef207eb4cbb427cc216380dac40902dabddeaada9f4ed2512f9b9613a7ced79d3996ad5050ca542f31032bd638193380c2d72f02965651ab4a26264253bb8a4ccb9b33afbc8c8b4f3e331baf50537b8ee80364038088aca3cf020235a7367536243629560b8a40f104352c89a2d4719e86f54175c2e4e3ecfec9938090cad2c60e023b623a01eace71e9b37d2bfac84f9aafc85dbf62a0f452446c5de0ca50cf910580e08d84ddcb010258c370ee02069943e5440294aeafae29656f9782b0a565d26065bb7af07a6af980c0caf384a30202899a53eeb05852a912bcbc6fa78e4c85f0b059726b0b8f0753e7aa54fc9d7ce82101351af203765d1679e2a9458ab6737d289e18c49766d41fc31a2bf0fe32dd196200".into(),
        block_header: BlockHeader {
            block_size: 345,
            block_weight: 345,
            cumulative_difficulty: 4646953,
            cumulative_difficulty_top64: 0,
            depth: 3166236,
            difficulty: 51263,
            difficulty_top64: 0,
            hash: "ff617b489e91f5db76f6f2cc9b3f03236e09fb191b4238f4a1e64185a6c28019".into(),
            height: 123,
            long_term_weight: 345,
            major_version: 1,
            miner_tx_hash: "054ba33024e72dfe8dafabc8af7c0e070e9aca3a9df44569cfa3c96669f9542f".into(),
            minor_version: 0,
            nonce: 3136465066,
            num_txes: 0,
            orphan_status: false,
            pow_hash: "".into(),
            prev_hash: "72244e0c8b2b8b99236e10c03eba53685b346aab525eb20b59a459b5935cd5a5".into(),
            reward: 17590122566891,
            timestamp: 1397823628,
            wide_cumulative_difficulty: "0x46e829".into(),
            wide_difficulty: "0xc83f".into(),
        },
        credits: 0,
        json: "{\n  \"major_version\": 1, \n  \"minor_version\": 0, \n  \"timestamp\": 1397823628, \n  \"prev_id\": \"72244e0c8b2b8b99236e10c03eba53685b346aab525eb20b59a459b5935cd5a5\", \n  \"nonce\": 3136465066, \n  \"miner_tx\": {\n    \"version\": 1, \n    \"unlock_time\": 183, \n    \"vin\": [ {\n        \"gen\": {\n          \"height\": 123\n        }\n      }\n    ], \n    \"vout\": [ {\n        \"amount\": 566891, \n        \"target\": {\n          \"key\": \"f917ac2dc38c0e0735f2c97df4a307a445b32abaf0ad528c385ae11a7e767d38\"\n        }\n      }, {\n        \"amount\": 2000000, \n        \"target\": {\n          \"key\": \"15a39af4cf4c67136ecc048d9296b4cb7a6be61275a0ef207eb4cbb427cc2163\"\n        }\n      }, {\n        \"amount\": 20000000, \n        \"target\": {\n          \"key\": \"dabddeaada9f4ed2512f9b9613a7ced79d3996ad5050ca542f31032bd6381933\"\n        }\n      }, {\n        \"amount\": 100000000, \n        \"target\": {\n          \"key\": \"965651ab4a26264253bb8a4ccb9b33afbc8c8b4f3e331baf50537b8ee8036403\"\n        }\n      }, {\n        \"amount\": 90000000000, \n        \"target\": {\n          \"key\": \"35a7367536243629560b8a40f104352c89a2d4719e86f54175c2e4e3ecfec993\"\n        }\n      }, {\n        \"amount\": 500000000000, \n        \"target\": {\n          \"key\": \"3b623a01eace71e9b37d2bfac84f9aafc85dbf62a0f452446c5de0ca50cf9105\"\n        }\n      }, {\n        \"amount\": 7000000000000, \n        \"target\": {\n          \"key\": \"58c370ee02069943e5440294aeafae29656f9782b0a565d26065bb7af07a6af9\"\n        }\n      }, {\n        \"amount\": 10000000000000, \n        \"target\": {\n          \"key\": \"899a53eeb05852a912bcbc6fa78e4c85f0b059726b0b8f0753e7aa54fc9d7ce8\"\n        }\n      }\n    ], \n    \"extra\": [ 1, 53, 26, 242, 3, 118, 93, 22, 121, 226, 169, 69, 138, 182, 115, 125, 40, 158, 24, 196, 151, 102, 212, 31, 195, 26, 43, 240, 254, 50, 221, 25, 98\n    ], \n    \"signatures\": [ ]\n  }, \n  \"tx_hashes\": [ ]\n}".into(),
        miner_tx_hash: "054ba33024e72dfe8dafabc8af7c0e070e9aca3a9df44569cfa3c96669f9542f".into(),
        status: "OK".into(),
        top_hash: "".into(),
        untrusted: false
    };

    let response = Response::ok(Id::Str("0".into()), payload);

    assert_ser_string_pretty(&response, RESPONSE);
    assert_de(RESPONSE, response);
}

/// Tests that Monero's `get_block_count` request and response
/// in JSON string form gets correctly (de)serialized.
#[test]
fn monero_jsonrpc_get_block_count() {
    //--- Request
    const REQUEST: &str = r#"{"jsonrpc":"2.0","id":0,"method":"get_block_count"}"#;
    let request = Request::new_with_id(Id::Num(0), Methods::GetBlockCount);
    assert_ser_string(&request, REQUEST);
    assert_de(REQUEST, request);

    //--- Response
    const RESPONSE: &str = r#"{
  "jsonrpc": "2.0",
  "id": 0,
  "result": {
    "count": 3166375,
    "status": "OK",
    "untrusted": false
  }
}"#;

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct Json {
        count: u64,
        status: String,
        untrusted: bool,
    }

    let payload = Json {
        count: 3166375,
        status: "OK".into(),
        untrusted: false,
    };

    let response = Response::ok(Id::Num(0), payload);

    assert_ser_string_pretty(&response, RESPONSE);
    assert_de(RESPONSE, response);
}
