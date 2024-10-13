use serde::{Deserialize, Serialize};

use crate::bytes::Bytes;

#[derive(Serialize, Deserialize)]
struct TxPoolAdd {
    version: u8,
    unlock_time: u64,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    extra: Bytes<44>,
    signatures: Vec<NotUsed>,
    ringct: RingCt,
}

#[derive(Serialize, Deserialize)]
struct Input {
    to_key: ToKey,
}

#[derive(Serialize, Deserialize)]
struct ToKey {
    amount: u64,
    key_offsets: Vec<u64>,
    key_image: Bytes<32>,
}

#[derive(Serialize, Deserialize)]
struct Output {
    amount: u64,
    to_tagged_key: ToTaggedKey,
}

#[derive(Serialize, Deserialize)]
struct ToTaggedKey {
    key: Bytes<32>,
    view_tag: Bytes<1>,
}

#[derive(Serialize, Deserialize)]
struct RingCt {
    #[serde(rename = "type")]
    ringct_type: u8,
    encrypted: Vec<Encrypted>,
    commitments: Vec<Bytes<32>>,
    fee: u64,
    prunable: Prunable,
}

#[derive(Serialize, Deserialize)]
struct Encrypted {
    mask: Bytes<32>,
    amount: Bytes<32>,
}

#[derive(Serialize, Deserialize)]
struct Prunable {
    range_proofs: Vec<NotUsed>,
    bulletproofs: Vec<NotUsed>,
    bulletproofs_plus: Vec<BulletproofPlus>,
    mlsags: Vec<Bytes<32>>,
    clsags: Vec<Clsag>,
    pseudo_outs: Vec<Bytes<32>>,
}

#[expect(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct BulletproofPlus {
    V: Vec<Bytes<32>>,
    A: Bytes<32>,
    A1: Bytes<32>,
    B: Bytes<32>,
    r1: Bytes<32>,
    s1: Bytes<32>,
    d1: Bytes<32>,
    L: Vec<Bytes<32>>,
    R: Vec<Bytes<32>>,
}

#[derive(Serialize, Deserialize)]
struct NotUsed;

#[expect(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct Clsag {
    s: Vec<Bytes<32>>,
    c1: Bytes<32>,
    D: Bytes<32>,
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use serde_json::{self, json};

    use crate::message_types::TxPoolAdd;

    #[test]
    fn test_txpooladd_json() {
        let json1 = json!([
          {
            "version": 2,
            "unlock_time": 0,
            "inputs": [
              {
                "to_key": {
                  "amount": 0,
                  "key_offsets": [
                    82773133,
                    30793552,
                    578803,
                    620532,
                    114291,
                    291870,
                    111275,
                    86455,
                    19769,
                    1238,
                    15164,
                    11374,
                    5240,
                    3547,
                    7423,
                    4198
                  ],
                  "key_image": "89c060b57bba20c0b795bda4b618749e04eba5b40b30062b071dff6e8dd9071d"
                }
              }
            ],
            "outputs": [
              {
                "amount": 0,
                "to_tagged_key": {
                  "key": "05b4ff4c3ced6ba078a078af8fee5916512a1893f2b6d9373fb90e0eb4040095",
                  "view_tag": "7a"
                }
              },
              {
                "amount": 0,
                "to_tagged_key": {
                  "key": "60250376bca49bf24cef45c12738b86347df10954cd35630e81b90bf01e922af",
                  "view_tag": "b8"
                }
              }
            ],
            "extra": "01154b87b3334ce9f99d04635eae4e31252a20ba22acb96ff0764a03dc91d203ed020901be80cbce0723d0b4",
            "signatures": [],
            "ringct": {
              "type": 6,
              "encrypted": [
                {
                  "mask": "0000000000000000000000000000000000000000000000000000000000000000",
                  "amount": "a956be1858615454000000000000000000000000000000000000000000000000"
                },
                {
                  "mask": "0000000000000000000000000000000000000000000000000000000000000000",
                  "amount": "72972be61af1210b000000000000000000000000000000000000000000000000"
                }
              ],
              "commitments": [
                "cc2a17e43f0b183235a06e8582fcaaa7c21a07732077e66d4dcfaa0db691ea20",
                "04e3cd1d3430bb7a1d9ede5ce9ec0ef2f6f9dd9fd31fb95c9e0b3148f1a660c8"
              ],
              "fee": 30660000,
              "prunable": {
                "range_proofs": [],
                "bulletproofs": [],
                "bulletproofs_plus": [
                  {
                    "V": [
                      "0196c1e9ba57ae053ae19c1bfd49e13146bd4b6e49401582f8a5a6f65ae560d0",
                      "aecd14b0e2d788315023601947c12d7e9227d8a1a0aee41f0b34fe196d96119f"
                    ],
                    "A": "8011fb75ba56d16b1ef1193e1fdfdb81e6b83afd726087427163857e8fcdf08e",
                    "A1": "ab91ab6863fbdee1fb71791e5297d007269f1b2cc050df40628ee7d0a1a5f3cb",
                    "B": "df1d082111b51d479b7fa72f6363bb731207c9343a528dc05b5798af56702521",
                    "r1": "2e212ae9ad704611a39b9b242453d2408045b303738b51d6f88f9dba06233401",
                    "s1": "36be53973fd971edff1f43cc5d04dda78d2b01f4caeaf38bbe195b04e309b30d",
                    "d1": "592116ca54b2d3ca0e9f222ffcc5fd63d3c992470473911fc70822f37672350a",
                    "L": [
                      "98f1e11d62b90c665a8a96fb1b10332e37a790ea1e01a9e8ec8de74b7b27b0df",
                      "3a14689f3d743a3be719df9af28ca2f0f398e3a2731d5d6f342d0485bf81a525",
                      "bcb9e389fd494db66e4c796ff03795daa131426c0776ded6d37bfae51f29623d",
                      "5aa7e1f2bfcfcd74ac8305ce59a7baf5a901f84f8fbdd3a2d639e4058f35e98b",
                      "5939aa7ea012f88a26bab20270ce5b164c1880f793dc249ec215a0783b4d4ca7",
                      "08286f78d1bb0d7fc2efc7a3ac314707a4a1ac9656656d496180e131c1748496",
                      "7fc1de780305601aab95fda4b005927a4643f222e28407c31ad46cc935b7a27c"
                    ],
                    "R": [
                      "69b4f329c0a5f8ae05891ac5ac35b947a7442b66e5b5693c99435deac3a62662",
                      "a193038cb8dc9d22abe6577fe44271c1693176cb636f9d101723670fb5ca5cda",
                      "90670e7083e503c2989b6548500234740dabf3451b0bd376979e03ca0cb5e50c",
                      "6ab149089f73799811f631eab272bd6c8f190f38efff4d49577364956d0148bf",
                      "62f2178cbdc760a0d3787b5fd42161c3c98394c2ff2b88efc039df59d2116e5d",
                      "536f91da278f730f2524260d2778dc5959d40a5c724dd789d35bbd309eabd933",
                      "e47c5c8181e692f3ad91733e7d9a52f8b7e3f5016c5e65f789eea367a13f16cd"
                    ]
                  }
                ],
                "mlsags": [],
                "clsags": [
                  {
                    "s": [
                      "f70840a8d65da85e962d2ce5ed1293ae3de83318b464363db85505d99e317b01",
                      "b7c1125be139b4ed201ce85b8453920306cac7c5da11e0f8c0fd7702f15c6a06",
                      "5a04335699f5a816eed1cab79085814dbcf3be5cef51b078b1c3e0210bbba606",
                      "e4743e114fd6352ea29e0b48ac96688edaba1d5d0634c34301756902eeb1fb0e",
                      "34aae87ab091082356d2815a7c8e973124245ebc6d163b9f01fbfeb360edcf04",
                      "d2d0b6ddb44ed42096affec08ea9cd77d2c7cdc5b2e1e964f836d3717640ec00",
                      "79b34258c8be04ddd955389f7ee3b912286c23492c519a5687b81d770619620e",
                      "3c889c19693463160d6c7e642c46f5d41db052ee3358c7dcb4826f48bca26607",
                      "da04927a438fd0d9674e64f0c016f30fde27f251d3466f29dcd5b3d757fec90c",
                      "f3e08d83b11ca6529bc18748d3f732c325fca8ff79f69f0ed754bcd529898102",
                      "f00d7125909a9a8cc5283ffc7727fce945e85828459eecb836c7aedca414350e",
                      "0a635a193af37be1c9519309f25eaf9f37b7bc5892864646d8d2a2187fcec601",
                      "0c4154d575dff3699bd41f0c354601de6535161755bd2164526076f37e2c6908",
                      "f7b21e2698333285ea10a95edbe80fe0bb8740c30b35c25bd2002e3693867e02",
                      "a637f338ff2ed65fa96e5529abc575fc2a35ed1a3f62a9e7be495069d8438800",
                      "f7c355f1c3a663978c5fe1c9337aabd4085ee537a61eec2c5c1e837cb3728c09"
                    ],
                    "c1": "c5dd25e0e32dbefa6ac1d0dc9072620eb97a99224462cdd163287f2b60b9810b",
                    "D": "c4fa3f939ccf02e4c8842cbd417cf3690421986e558734a0a029f8a86d2791a8"
                  }
                ],
                "pseudo_outs": [
                  "bcb08920f5476d74294aeb89c8001123bffd2f2ab84e105d553b807674c595ce"
                ]
              }
            }
          }
        ]);

        let tx_pool_adds: Vec<TxPoolAdd> = serde_json::from_value(json1.clone()).unwrap();
        let json2 = serde_json::to_value(&tx_pool_adds).unwrap();
        assert_json_eq!(json1, json2);
    }
}
