//! JSON block types.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use monero_serai::{block, transaction};

use cuprate_helper::cast::usize_to_u64;

use crate::{
    hex::{HexBytes1, HexBytes32},
    json::output::{Output, TaggedKey, Target},
};

/// JSON representation of a block.
///
/// Used in:
/// - [`/get_block` -> `json`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block)
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Block {
    pub major_version: u8,
    pub minor_version: u8,
    pub timestamp: u64,
    pub prev_id: HexBytes32,
    pub nonce: u32,
    pub miner_tx: MinerTransaction,
    pub tx_hashes: Vec<HexBytes32>,
}

impl From<block::Block> for Block {
    fn from(b: block::Block) -> Self {
        let Ok(miner_tx) = MinerTransaction::try_from(b.miner_transaction) else {
            unreachable!("input is a miner tx, this should never fail");
        };

        let tx_hashes = b.transactions.into_iter().map(HexBytes32).collect();

        Self {
            major_version: b.header.hardfork_version,
            minor_version: b.header.hardfork_signal,
            timestamp: b.header.timestamp,
            prev_id: HexBytes32(b.header.previous),
            nonce: b.header.nonce,
            miner_tx,
            tx_hashes,
        }
    }
}

/// [`Block::miner_tx`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[serde(untagged)]
pub enum MinerTransaction {
    V1 {
        /// This field is [flattened](https://serde.rs/field-attrs.html#flatten).
        #[serde(flatten)]
        prefix: MinerTransactionPrefix,
        signatures: [(); 0],
    },
    V2 {
        /// This field is [flattened](https://serde.rs/field-attrs.html#flatten).
        #[serde(flatten)]
        prefix: MinerTransactionPrefix,
        rct_signatures: MinerTransactionRctSignatures,
    },
}

impl TryFrom<transaction::Transaction> for MinerTransaction {
    type Error = transaction::Transaction;

    /// # Errors
    /// This function errors if the input is not a miner transaction.
    fn try_from(tx: transaction::Transaction) -> Result<Self, transaction::Transaction> {
        fn map_prefix(
            prefix: transaction::TransactionPrefix,
            version: u8,
        ) -> Result<MinerTransactionPrefix, transaction::TransactionPrefix> {
            let Some(input) = prefix.inputs.first() else {
                return Err(prefix);
            };

            let height = match input {
                transaction::Input::Gen(height) => usize_to_u64(*height),
                transaction::Input::ToKey { .. } => return Err(prefix),
            };

            let vin = {
                let r#gen = Gen { height };
                let input = Input { r#gen };
                [input]
            };

            let vout = prefix
                .outputs
                .into_iter()
                .map(|o| {
                    let amount = o.amount.unwrap_or(0);

                    let target = match o.view_tag {
                        Some(view_tag) => {
                            let tagged_key = TaggedKey {
                                key: HexBytes32(o.key.0),
                                view_tag: HexBytes1([view_tag]),
                            };

                            Target::TaggedKey { tagged_key }
                        }
                        None => Target::Key {
                            key: HexBytes32(o.key.0),
                        },
                    };

                    Output { amount, target }
                })
                .collect();

            let unlock_time = match prefix.additional_timelock {
                transaction::Timelock::None => 0,
                transaction::Timelock::Block(x) => usize_to_u64(x),
                transaction::Timelock::Time(x) => x,
            };

            Ok(MinerTransactionPrefix {
                version,
                unlock_time,
                vin,
                vout,
                extra: prefix.extra,
            })
        }

        Ok(match tx {
            transaction::Transaction::V1 { prefix, signatures } => {
                let prefix = match map_prefix(prefix, 1) {
                    Ok(p) => p,
                    Err(prefix) => return Err(transaction::Transaction::V1 { prefix, signatures }),
                };

                Self::V1 {
                    prefix,
                    signatures: [(); 0],
                }
            }
            transaction::Transaction::V2 { prefix, proofs } => {
                let prefix = match map_prefix(prefix, 2) {
                    Ok(p) => p,
                    Err(prefix) => return Err(transaction::Transaction::V2 { prefix, proofs }),
                };

                Self::V2 {
                    prefix,
                    rct_signatures: MinerTransactionRctSignatures { r#type: 0 },
                }
            }
        })
    }
}

impl Default for MinerTransaction {
    fn default() -> Self {
        Self::V1 {
            prefix: Default::default(),
            signatures: Default::default(),
        }
    }
}

/// [`MinerTransaction::V1::prefix`] & [`MinerTransaction::V2::prefix`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MinerTransactionPrefix {
    pub version: u8,
    pub unlock_time: u64,
    pub vin: [Input; 1],
    pub vout: Vec<Output>,
    pub extra: Vec<u8>,
}

/// [`MinerTransaction::V2::rct_signatures`].
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MinerTransactionRctSignatures {
    pub r#type: u8,
}

/// [`MinerTransactionPrefix::vin`].
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Input {
    pub r#gen: Gen,
}

/// [`Input::gen`].
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Gen {
    pub height: u64,
}

#[cfg(test)]
mod test {
    use hex_literal::hex;
    use pretty_assertions::assert_eq;

    use super::*;

    #[expect(clippy::needless_pass_by_value)]
    fn test(block: Block, block_json: &'static str) {
        let json = serde_json::from_str::<Block>(block_json).unwrap();
        assert_eq!(block, json);
        let string = serde_json::to_string(&json).unwrap();
        assert_eq!(block_json, &string);
    }

    #[test]
    fn block_300000() {
        const JSON: &str = r#"{"major_version":1,"minor_version":0,"timestamp":1415690591,"prev_id":"e97a0ab6307de9b9f9a9872263ef3e957976fb227eb9422c6854e989e5d5d34c","nonce":2147484616,"miner_tx":{"version":1,"unlock_time":300060,"vin":[{"gen":{"height":300000}}],"vout":[{"amount":47019296802,"target":{"key":"3c1dcbf5b485987ecef4596bb700e32cbc7bd05964e3888ffc05f8a46bf5fc33"}},{"amount":200000000000,"target":{"key":"5810afc7a1b01a1c913eb6aab15d4a851cbc4a8cf0adf90bb80ac1a7ca9928aa"}},{"amount":3000000000000,"target":{"key":"520f49c5f2ce8456dc1a565f35ed3a5ccfff3a1210b340870a57d2749a81a2df"}},{"amount":10000000000000,"target":{"key":"44d7705e62c76c2e349a474df6724aa1d9932092002b03a94f9c19d9d12b9427"}}],"extra":[1,251,8,189,254,12,213,173,108,61,156,198,144,151,31,130,141,211,120,55,81,98,32,247,111,127,254,170,170,240,124,190,223,2,8,0,0,0,64,184,115,46,246],"signatures":[]},"tx_hashes":[]}"#;

        let block = Block {
            major_version: 1,
            minor_version: 0,
            timestamp: 1415690591,
            prev_id: HexBytes32(hex!(
                "e97a0ab6307de9b9f9a9872263ef3e957976fb227eb9422c6854e989e5d5d34c"
            )),
            nonce: 2147484616,
            miner_tx: MinerTransaction::V1 {
                prefix: MinerTransactionPrefix {
                    version: 1,
                    unlock_time: 300060,
                    vin: [Input {
                        r#gen: Gen { height: 300000 },
                    }],
                    vout: vec![
                      Output {
                        amount: 47019296802,
                        target: Target::Key {
                          key: HexBytes32(hex!("3c1dcbf5b485987ecef4596bb700e32cbc7bd05964e3888ffc05f8a46bf5fc33")),
                        }
                      },
                      Output {
                        amount: 200000000000,
                        target: Target::Key {
                          key: HexBytes32(hex!("5810afc7a1b01a1c913eb6aab15d4a851cbc4a8cf0adf90bb80ac1a7ca9928aa")),
                        }
                      },
                      Output {
                        amount: 3000000000000,
                        target: Target::Key {
                          key: HexBytes32(hex!("520f49c5f2ce8456dc1a565f35ed3a5ccfff3a1210b340870a57d2749a81a2df")),
                        }
                      },
                      Output {
                        amount: 10000000000000,
                        target: Target::Key {
                          key: HexBytes32(hex!("44d7705e62c76c2e349a474df6724aa1d9932092002b03a94f9c19d9d12b9427")),
                        }
                      }
                    ],
                    extra: vec![
                        1, 251, 8, 189, 254, 12, 213, 173, 108, 61, 156, 198, 144, 151, 31, 130,
                        141, 211, 120, 55, 81, 98, 32, 247, 111, 127, 254, 170, 170, 240, 124, 190,
                        223, 2, 8, 0, 0, 0, 64, 184, 115, 46, 246,
                    ],
                },
                signatures: [],
            },
            tx_hashes: vec![],
        };

        test(block, JSON);
    }

    #[test]
    fn block_3245409() {
        const JSON: &str = r#"{"major_version":16,"minor_version":16,"timestamp":1727293028,"prev_id":"41b56c273d69def3294e56179de71c61808042d54c1e085078d21dbe99e81b6f","nonce":311,"miner_tx":{"version":2,"unlock_time":3245469,"vin":[{"gen":{"height":3245409}}],"vout":[{"amount":601012280000,"target":{"tagged_key":{"key":"8c0b16c6df02b9944b49f375d96a958a0fc5431c048879bb5bf25f64a1163b9e","view_tag":"88"}}}],"extra":[1,39,23,182,203,58,48,15,217,9,13,147,104,133,206,176,185,56,237,179,136,72,84,129,113,98,206,4,18,50,130,162,94,2,17,73,18,21,33,32,112,5,0,0,0,0,0,0,0,0,0,0],"rct_signatures":{"type":0}},"tx_hashes":["eab76986a0cbcae690d8499f0f616f783fd2c89c6f611417f18011950dbdab2e","57b19aa8c2cdbb6836cf13dd1e321a67860965c12e4418f3c30f58c8899a851e","5340185432ab6b74fb21379f7e8d8f0e37f0882b2a7121fd7c08736f079e2edc","01dc6d31db56d68116f5294c1b4f80b33b048b5cdfefcd904f23e6c0de3daff5","c9fb6a2730678203948fef2a49fa155b63f35a3649f3d32ed405a6806f3bbd56","af965cdd2a2315baf1d4a3d242f44fe07b1fd606d5f4853c9ff546ca6c12a5af","97bc9e047d25fae8c14ce6ec882224e7b722f5e79b62a2602a6bacebdac8547b","28c46992eaf10dc0cceb313c30572d023432b7bd26e85e679bc8fe419533a7bf","c32e3acde2ff2885c9cc87253b40d6827d167dfcc3022c72f27084fd98788062","19e66a47f075c7cccde8a7b52803119e089e33e3a4847cace0bd1d17b0d22bab","8e8ac560e77a1ee72e82a5eb6887adbe5979a10cd29cb2c2a3720ce87db43a70","b7ff5141524b5cca24de6780a5dbfdf71e7de1e062fd85f557fb3b43b8e285dc","f09df0f113763ef9b9a2752ac293b478102f7cab03ef803a3d9db7585aea8912"]}"#;

        let block = Block {
            major_version: 16,
            minor_version: 16,
            timestamp: 1727293028,
            prev_id: HexBytes32(hex!(
                "41b56c273d69def3294e56179de71c61808042d54c1e085078d21dbe99e81b6f"
            )),
            nonce: 311,
            miner_tx: MinerTransaction::V2 {
                prefix: MinerTransactionPrefix {
                    version: 2,
                    unlock_time: 3245469,
                    vin: [Input {
                        r#gen: Gen { height: 3245409 },
                    }],
                    vout: vec![Output {
                        amount: 601012280000,
                        target: Target::TaggedKey {
                            tagged_key: TaggedKey {
                                key: HexBytes32(hex!(
                                "8c0b16c6df02b9944b49f375d96a958a0fc5431c048879bb5bf25f64a1163b9e"
                            )),
                                view_tag: HexBytes1(hex!("88")),
                            },
                        },
                    }],
                    extra: vec![
                        1, 39, 23, 182, 203, 58, 48, 15, 217, 9, 13, 147, 104, 133, 206, 176, 185,
                        56, 237, 179, 136, 72, 84, 129, 113, 98, 206, 4, 18, 50, 130, 162, 94, 2,
                        17, 73, 18, 21, 33, 32, 112, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ],
                },
                rct_signatures: MinerTransactionRctSignatures { r#type: 0 },
            },
            tx_hashes: vec![
                HexBytes32(hex!(
                    "eab76986a0cbcae690d8499f0f616f783fd2c89c6f611417f18011950dbdab2e"
                )),
                HexBytes32(hex!(
                    "57b19aa8c2cdbb6836cf13dd1e321a67860965c12e4418f3c30f58c8899a851e"
                )),
                HexBytes32(hex!(
                    "5340185432ab6b74fb21379f7e8d8f0e37f0882b2a7121fd7c08736f079e2edc"
                )),
                HexBytes32(hex!(
                    "01dc6d31db56d68116f5294c1b4f80b33b048b5cdfefcd904f23e6c0de3daff5"
                )),
                HexBytes32(hex!(
                    "c9fb6a2730678203948fef2a49fa155b63f35a3649f3d32ed405a6806f3bbd56"
                )),
                HexBytes32(hex!(
                    "af965cdd2a2315baf1d4a3d242f44fe07b1fd606d5f4853c9ff546ca6c12a5af"
                )),
                HexBytes32(hex!(
                    "97bc9e047d25fae8c14ce6ec882224e7b722f5e79b62a2602a6bacebdac8547b"
                )),
                HexBytes32(hex!(
                    "28c46992eaf10dc0cceb313c30572d023432b7bd26e85e679bc8fe419533a7bf"
                )),
                HexBytes32(hex!(
                    "c32e3acde2ff2885c9cc87253b40d6827d167dfcc3022c72f27084fd98788062"
                )),
                HexBytes32(hex!(
                    "19e66a47f075c7cccde8a7b52803119e089e33e3a4847cace0bd1d17b0d22bab"
                )),
                HexBytes32(hex!(
                    "8e8ac560e77a1ee72e82a5eb6887adbe5979a10cd29cb2c2a3720ce87db43a70"
                )),
                HexBytes32(hex!(
                    "b7ff5141524b5cca24de6780a5dbfdf71e7de1e062fd85f557fb3b43b8e285dc"
                )),
                HexBytes32(hex!(
                    "f09df0f113763ef9b9a2752ac293b478102f7cab03ef803a3d9db7585aea8912"
                )),
            ],
        };

        test(block, JSON);
    }
}
