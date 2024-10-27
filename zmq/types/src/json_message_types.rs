//! Objects for JSON serialization and deserialization in message bodies of
//! the ZMQ pub/sub interface. Handles JSON for the following subscriptions:
//! * `json-full-txpool_add` (`Vec<TxPoolAdd>`)
//! * `json-minimal-txpool_add` (`Vec<TxPoolAddMin>`)
//! * `json-full-chain_main` (`Vec<ChainMain>`)
//! * `json-minimal-chain_main` (`ChainMainMin`)
//! * `json-full-miner_data` (`MinerData`)
use serde::{Deserialize, Serialize};

use crate::{bytes_hex::Bytes, u128_hex::U128};

/// ZMQ `json-full-txpool_add` packets contain an array of `TxPoolAdd`.
///
/// Each `TxPoolAdd` object represents a new transaction in the mempool that was
/// not previously seen in a block. Miner coinbase transactions *are not*
/// included. `do-not-relay` transactions *are* included. Values are not
/// republished during a re-org.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TxPoolAdd {
    pub version: u8,
    // If not 0, this is the block height when a transaction output is spendable.
    pub unlock_time: u64,
    pub inputs: Vec<PoolInput>,
    pub outputs: Vec<Output>,
    pub extra: Bytes<44>,
    signatures: Vec<Obsolete>,
    pub ringct: PoolRingCt,
}

/// ZMQ `json-minimal-txpool_add` subscriber messages contain an array of
/// `TxPoolAddMin` JSON objects. See `TxPoolAdd` for information on which
/// transactions are published to subscribers.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TxPoolAddMin {
    pub id: Bytes<32>,
    // size of the full transaction blob
    pub blob_size: u64,
    pub weight: u64,
    // mining fee included in the transaction in piconeros
    pub fee: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainMain {
    /// The major version of the monero protocol at the next? block height.
    pub major_version: u8,
    /// The minor version of the monero protocol at the next? block height.
    pub minor_version: u8,
    /// Unix epoch time, decided by the miner, at which the block was mined.
    pub timestamp: u64,
    /// Hash of the most recent block on which to mine the next block
    pub prev_id: Bytes<32>,
    /// a cryptographic random one-time number used in mining a Monero block
    pub nonce: u32,
    /// Miner transaction information
    pub miner_tx: MinerTx,
    /// List of hashes of non-coinbase transactions in the block. If there are no
    /// other transactions, this will be an empty list.
    pub tx_hashes: Vec<Bytes<32>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ChainMainMin {
    pub first_height: u64,
    pub first_prev_id: Bytes<32>,
    pub ids: Vec<Bytes<32>>,
}

/// ZMQ `json-full-miner_data` subscriber messages contain a single
/// `FullMinerData` object that provides the necessary data to create a
/// custom block template. There is no min version of this object.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MinerData {
    pub major_version: u8,
    pub height: u64,
    pub prev_id: Bytes<32>,
    pub seed_hash: Bytes<32>,
    // Least-significant 64 bits of the 128-bit network difficulty
    pub difficulty: U128,
    // Median adjusted block size of latest 100000 blocks
    pub median_weight: u64,
    /// Fixed at `u64::MAX` in perpetuity as Monero has already reached tail emission.
    /// Note that not all JSON parsers can handle this large value correctly.
    pub already_generated_coins: u64,
    /// List of mineable mempool transactions
    pub tx_backlog: Vec<TxBacklog>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PoolInput {
    pub to_key: ToKey,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MinerInput {
    pub r#gen: Gen,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Gen {
    pub height: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ToKey {
    pub amount: u64,
    pub key_offsets: Vec<u64>,
    pub key_image: Bytes<32>,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Output {
    pub amount: u64,
    pub to_tagged_key: ToTaggedKey,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct ToTaggedKey {
    pub key: Bytes<32>,
    pub view_tag: Bytes<1>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PoolRingCt {
    pub r#type: u8,
    pub encrypted: Vec<Encrypted>,
    pub commitments: Vec<Bytes<32>>,
    pub fee: u64,
    pub prunable: Prunable,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MinerRingCt {
    pub r#type: u8,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Encrypted {
    pub mask: Bytes<32>,
    pub amount: Bytes<32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Prunable {
    range_proofs: Vec<Obsolete>,
    bulletproofs: Vec<Obsolete>,
    pub bulletproofs_plus: Vec<BulletproofPlus>,
    pub mlsags: Vec<Bytes<32>>,
    pub clsags: Vec<Clsag>,
    pub pseudo_outs: Vec<Bytes<32>>,
}

#[expect(non_snake_case)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BulletproofPlus {
    pub V: Vec<Bytes<32>>,
    pub A: Bytes<32>,
    pub A1: Bytes<32>,
    pub B: Bytes<32>,
    pub r1: Bytes<32>,
    pub s1: Bytes<32>,
    pub d1: Bytes<32>,
    pub L: Vec<Bytes<32>>,
    pub R: Vec<Bytes<32>>,
}

/// Placeholder element type so obsolete fields can be deserialized
/// to the empty vector for backwards compatibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Obsolete;

#[expect(non_snake_case)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Clsag {
    pub s: Vec<Bytes<32>>,
    pub c1: Bytes<32>,
    pub D: Bytes<32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinerTx {
    pub version: u8,
    /// The block height when the coinbase transaction becomes spendable
    pub unlock_time: u64,
    /// list of transaction inputs
    pub inputs: Vec<MinerInput>,
    /// list of transaction outputs
    pub outputs: Vec<Output>,
    pub extra: Bytes<52>,
    signatures: Vec<Obsolete>,
    pub ringct: MinerRingCt,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TxBacklog {
    pub id: Bytes<32>,
    pub weight: u64,
    pub fee: u64,
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;
    use serde_json::{self, json};

    use super::*;

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

    #[test]
    fn test_txpooladd_min_json() {
        let json1 = json!([
          {
            "id": "b5086746e805d875cbbbbb49e19aac29d9b75019f656fab8516cdf64ac5cd346",
            "blob_size": 1533,
            "weight": 1533,
            "fee": 30660000
          }
        ]);

        let tx_pool_adds: Vec<TxPoolAddMin> = serde_json::from_value(json1.clone()).unwrap();
        let json2 = serde_json::to_value(&tx_pool_adds).unwrap();
        assert_json_eq!(json1, json2);
    }

    #[test]
    fn test_chain_main_json() {
        let json1 = json!([
          {
            "major_version": 16,
            "minor_version": 16,
            "timestamp": 1726973843,
            "prev_id": "ce3731311b7e4c1e58a2fe902dbb5c60bb2c0decc163d5397fa52a260d7f09c1",
            "nonce": 537273946,
            "miner_tx": {
              "version": 2,
              "unlock_time": 3242818,
              "inputs": [
                {
                  "gen": {
                    "height": 3242758
                  }
                }
              ],
              "outputs": [
                {
                  "amount": 618188180000_u64,
                  "to_tagged_key": {
                    "key": "83faf44df7e9fb4cf54a8dd6a63868507d1a1896bdb35ea9110d739d5da6cf21",
                    "view_tag": "38"
                  }
                }
              ],
              "extra": "010e3356a86dbb339354afbc693408dfe8648bffd0b276e6a431861eb73643d88d02115162e362c98e2d00000000000000000000",
              "signatures": [],
              "ringct": {
                "type": 0
              }
            },
            "tx_hashes": [
              "2c1b67d3f10b21270cac116e6d5278dc4024ee2d727e4ad56d6dedb1abc0270c",
              "c2cfec0de23229a2ab80ca464cef66fc1cad53647a444f048834ec236c38c867",
              "03c7649af2373c0f739d3c2eff9ee1580986b460d2abdd5e2aa332281e52da7e",
              "1e0834cc658599e786040bdcd9b589a5e8d975233b72279d04ece1a3dd5572b0",
              "ba65c30150e906a8799ee99bb2e6481873e42ed8b025cf967c5798528ddc81b4",
              "6fc7b1da1cf433edafb142173e9ac13fe05142a36d8a72e9efdf7a3b94da11d6",
              "847c06dcda4540d45cae868d4d031781bd87d9bfa4b2186a611428f52e68ccee",
              "79f87a1b2fc17295d2cf25b6a65dd17fd8630829ee50f9c48f15e4a24e72d872",
              "32b4f7ce6d864006b274dbd73fc8058151d0fd2dd0bb4b423120e32451fd59eb",
              "430fe7fa00b63b68b301a4e4810bef2b5be1f651dba8c360e86eac61227382e7",
              "9f8d2bf5e39071abccb336404ea72ab85cb731500a1d386a3bf537b9046df29d",
              "f63893b8226ca28c290cb65541d60c1675dda1e2a77a629b6b2d7c3886240b23",
              "ee8608b6e80cce14beaf95f747f4da8e40e40a49ad1adc20038843a6da3df3c6",
              "05783765c150ed1e46d6380726e7ca1f788305754e553f5f2d49b9f09aaaf88d",
              "20b4b95e62f45b72014d6ab14edb0b31e273cdc8c8d106068dd32ef6e92fc0a2",
              "9230fb0a9dce8e2ca7e109ebf3480838251691de8ed73ea91f74723c5cf19bac",
              "d59cf84a25f56ec0f1352bb05645efe9b9326598c4f7c5bc39a87eb7a20c48fc",
              "465deb73c48a460df71861d61666dabb906648035a1fecfd0e988ee37616c655",
              "5767bc633729ba4555561510f3db739431b16744234dcd549a0d346eaa6685b1",
              "2c8d9af5d5774de96e67835ac5adbc6ca5579125b08bc907b395645eea6410ec",
              "d385c884a0687c3360725dd3a3f6acf6f64bf38d8eeea1644d80bc23b13ee870",
              "b2bc7e9fa9c1da08a8b6ee58505611c05bc388fd30aece00e9a0041470f7e950",
              "69a4a79b50d42d372e91c6608c2652d1d5ddd343526c387ef6cf1e3c158b1765",
              "ef508dfa79bbedd226835c42a9d000a64cc4abe0250c9aa55fd968224e2b45c3",
              "0413c3b3fc621c472e10a102d77456db506f0df10a909833aed0c6738fb31eeb",
              "e0c52d6d649c2f1abce4c6ffce4dd75a23308afbb6abe33af53da232c40caf5f",
              "cd1fd68d2a15002ca6236083ff6ae165c8fd922f410da79640a4342fd8ebd1c8",
              "ba746f80ca4ee496f4188ba278f1ed69a913238450d52bd2e2f3d3bf6fdd43d3",
              "13c964bc13a55621b7bbbfe9a6d703536d951bfa19eedee93dd1286020959021",
              "41a6f8d0df227a401a9bd6f5c0fbc21ed89f515ea5c8434a087e8b880080ee1f",
              "41c2b5994284790b1ee158f7b87aa1231c14975d6456a91ff6f93c6f81277965",
              "7e6b7f169cc6cab88e652771157cf8c2eb6f69dffb6939a79b34c6554fe6c00b",
              "619517d9d138bf95c6b77eb801526b8419616de2b8618ccfd3b6d1c10364bc64",
              "52cca64fb20fc2f6d06034a1a2d9b5665972ebc2569ec69f8d473caada309add",
              "219c106d09da5a27b339ea0f070da090779b31ef9ccfa90d6d25e7388341eff9",
              "e07ce6e96e73cff80c9cc4c1b349ad1ef53cff210b876d4e7afd89fcc8b2e5dd",
              "e98f2a00b2892cd65c0252d956d88a4bb8024c7db98ca003c127b097f097f276",
              "ed81aa398071fe495e37095e51ff50053e132bd11f27ba9c06ac4bf4063b756f",
              "667d29a0cefa311e06fcfc22c98ef75edf81deb6c8a812492eb255a049c826db",
              "8b16e8cbc1765247456bd67a3106498f686401b7529dc0f6b03360caf8671135",
              "013e443e63259748f6d1a5653374826618ba066b7febcf55c829333f0dd9a6c3",
              "517a05d82de59a973eb4d343c45558841c9165ccd75ca7c9d2e1a35f80c26c15",
              "af74d5dd44cfed8f40f853a6fc405dae23d547482296f8dbbc13c1aed2c3d8c5",
              "b5086746e805d875cbbbbb49e19aac29d9b75019f656fab8516cdf64ac5cd346",
              "cfcda18d058656797a1272b384774dcfc26a504a24298aa49ba060eb6b4a19e0",
              "1f380660a99030cc45f85ba8ee0e0541035c0fde719c84aa692796328974c9dd",
              "53127181a0301a27b3a2749dc997556b211d949a99aa34d1c52d5c54220f49d2",
              "5d50a66df97f4decc4ecc3f5030589ef966d5af84a995f7fb14f1c02ae9704db",
              "cdab9628acdb57c460e292660e7a07caf2ddbcffdfff92f3e5e4fb12119a11ca",
              "e740a098a74d7a66a821c4ac3c5f913a82fc7445b5593cc5fa3e48ad1b4589b1",
              "760549176fec210cfe0ff58eabbf2670cf33b4cd3942a3b60a98bf8f328a6d01",
              "961b0956aa6303ed8ca1687d93ed46b9aa8a0203ec4ce0cbc2e86b364fbfb613",
              "b9db041b2c3bfc6b5b0facb638b0b4643eec76b060039a6b11fb43682ed77a97",
              "1011c321eb386b9975e8124bdb130790dcf4ac0021da3103cabbf7dfa18ccea7",
              "6a9d3d15be4b25bd544d96bb1d7685e53f9484735bb22994feffb9037009aeeb",
              "bf20d6193890cf7fdead9e3b60197564c663b5a62eda782a49d4aa7819bb9665",
              "472d28f9d25a95e625eb808ff3827e7f6792009e1ba0b3b21951f3058b65a75d",
              "e3931b2b66da07f983d2235d9d0b3a3098008458bdc0c1ad4370fae73e1eaa9e",
              "e18a0dea6382c95aa4089a971190683b171e9405c06fd4111924144600f3bcf3",
              "1a336bcf24026307821b76b9ca18b178c285c591c5df9906e3ffbd2050ccd356",
              "8ca2d0e5ae9b9981bb8b76ba0da383c585664b2a2f4e861d58aab00c9b0cc808",
              "e1866c27023ccea276034c4d572eab42713132e4fdb2aafa9488f6d74cd49303",
              "3674cfafba4cdea5775a72a82e5d553bf180beab456b3cbaa7b41a1574fe1948",
              "9bb400dd317425f40176c3094a5573037b0217e0b60761cb66a8fa15b63b36c3",
              "c078048028aca3e9bc40f68f4d42ef25c6af2cef4da20bf3be70dd6a23b82d52",
              "c28cc85f945085e70259ed02131ae3f8c5992e789c9c75c2c6e257306beaf26e",
              "4c2b121795fe2b90fda84813543952382daa29c7b96edd9f96040df13e48e347",
              "63c6fba30b5471fd60e715cbaf4448badafde68dbc42c54d96b56dd2c4bf2d15",
              "a4240138ecfe736113581f318f261a01992eaa8fa5b7bd6938d9dbeb65aa85d7",
              "b9d088a7b21f655d0cf50f8404e874f4d1655fb5565a354d2c0dd6d113619c66",
              "9133e7e98a83f6e10a7fd44c104d9124d93e0d3e920f5c160873b394dd3a2fcb",
              "953985dbd0ea6f86746e83be144ec2ff2897ef1f3506eede083b893e98dd63ea",
              "83af840c4cad46de96c86fcf700ade32e73260d4a16cefa330cb5a722ef59fdf",
              "eea3c0c2b016ea0c269f954fd8172c3d118f08103c9842b81b05290c9faf3780",
              "ac43a363fdb81fa4f6df1cb06ba49a5f4eeef411957cf2afad55cbc1e79bc4d1",
              "ca72cf7bda22aed15c16ca67e7b6cc57109cdc86d4ffe38fd71210a5380fcada",
              "477dc1cd62106d9df6b37f8515579a48d01b310387087c08ce7062a8eb5df98d",
              "d47b6dcd3b13288825c954df6c6e30eb683d1f79434beaee7172082f8ae74280",
              "9c64ef20c69589c56fcc5f3a0d10f6957ecea248e44acb432aaf16a88eeef946",
              "d2aa256bfd61bdb64ac38da6cbc3e77fb315bb9fbaf422087c10345377df44f6",
              "8b9623e4513594a6eaeb3475ea7d0eb585dd8f6e20e21c316db0b942fada2336",
              "860725ed0bd18c744e6b8b02888ad88be1cf23d7153131b220a0f9fbb76976bf",
              "387cc6e807efc263a0ad6a30e6313a27d16abef038264d0afa0e6ad943be55da"
            ]
          }
        ]);

        let chain_main: Vec<ChainMain> = serde_json::from_value(json1.clone()).unwrap();
        let json2 = serde_json::to_value(&chain_main).unwrap();
        assert_json_eq!(json1, json2);
    }

    #[test]
    fn test_chain_main_min_json() {
        let json1 = json!({
          "first_height": 3242758,
          "first_prev_id": "ce3731311b7e4c1e58a2fe902dbb5c60bb2c0decc163d5397fa52a260d7f09c1",
          "ids": [
            "ee1238b884e64f7e438223aa8d42d0efc15e7640f1a432448fbad116dc72f1b2"
          ]
        });

        let chain_main_min: ChainMainMin = serde_json::from_value(json1.clone()).unwrap();
        let json2 = serde_json::to_value(&chain_main_min).unwrap();
        assert_json_eq!(json1, json2);
    }

    #[test]
    fn test_miner_data_json() {
        let json1 = json!({
          "major_version": 16,
          "height": 3242764,
          "prev_id": "dc53c24683dca14586fb2909b9aa4a44adb524e010d438e2491e7d8cc1c80831",
          "seed_hash": "526577d6e6689ba8736c16ccc76e6ce4ada3b0ceeaa3a2260b96ba188a17d705",
          "difficulty": "0x526f2623ce",
          "median_weight": 300000,
          "already_generated_coins": 18446744073709551615_u64,
          "tx_backlog": [
            {
              "id": "dbec64651bb4e83d0e9a05c2826bde605a940f12179fab0ab5dc8bc4392c776b",
              "weight": 2905,
              "fee": 929600000
            },
            {
              "id": "ec5728dd1fbd98db1f93d612826e73b95f52cca49f247a6dbc35390f45766a7d",
              "weight": 2222,
              "fee": 44440000
            },
            {
              "id": "41f613b1a470af494e0a705993e305dfaad3e365fcc0b0db0118256fc54559aa",
              "weight": 2221,
              "fee": 44420000
            },
            {
              "id": "34fa33bf96dc2f825fe870e8f5402be6225c1623b345224e0dbc38b6407873de",
              "weight": 2217,
              "fee": 709440000
            }
          ]
        });

        let miner_data: MinerData = serde_json::from_value(json1.clone()).unwrap();
        let json2 = serde_json::to_value(&miner_data).unwrap();
        assert_json_eq!(json1, json2);
    }
}
