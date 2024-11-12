//! JSON transaction types.

#![expect(
    non_snake_case,
    reason = "JSON serialization requires non snake-case casing"
)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_helper::cast::usize_to_u64;

use monero_serai::{ringct, transaction};

use crate::{
    hex::HexBytes,
    json::output::{Output, TaggedKey, Target},
};

/// JSON representation of a non-miner transaction.
///
/// Used in:
/// - [`/get_transactions` -> `txs.as_json`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_transactions)
/// - [`/get_transaction_pool` -> `tx_json`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_transaction_pool)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Transaction {
    V1 {
        /// This field is [flattened](https://serde.rs/field-attrs.html#flatten).
        #[cfg_attr(feature = "serde", serde(flatten))]
        prefix: TransactionPrefix,
        signatures: Vec<HexBytes<64>>,
    },
    V2 {
        /// This field is [flattened](https://serde.rs/field-attrs.html#flatten).
        #[cfg_attr(feature = "serde", serde(flatten))]
        prefix: TransactionPrefix,
        rct_signatures: RctSignatures,
        /// This field is [`Some`] if [`Self::V2::rct_signatures`]
        /// is [`RctSignatures::NonCoinbase`], else [`None`].
        rctsig_prunable: Option<RctSigPrunable>,
    },
}

/// [`Transaction::V1::prefix`] & [`Transaction::V2::prefix`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionPrefix {
    pub version: u8,
    pub unlock_time: u64,
    pub vin: Vec<Input>,
    pub vout: Vec<Output>,
    pub extra: Vec<u8>,
}

impl From<transaction::Transaction> for Transaction {
    fn from(tx: transaction::Transaction) -> Self {
        fn map_prefix(prefix: transaction::TransactionPrefix, version: u8) -> TransactionPrefix {
            let mut height = 0;

            let vin = prefix
                .inputs
                .into_iter()
                .filter_map(|input| match input {
                    transaction::Input::ToKey {
                        amount,
                        key_offsets,
                        key_image,
                    } => {
                        let key = Key {
                            amount: amount.unwrap_or(0),
                            key_offsets,
                            k_image: HexBytes::<32>(key_image.compress().0),
                        };

                        Some(Input { key })
                    }
                    transaction::Input::Gen(h) => {
                        height = usize_to_u64(h);
                        None
                    }
                })
                .collect();

            let vout = prefix
                .outputs
                .into_iter()
                .map(|o| {
                    let amount = o.amount.unwrap_or(0);

                    let target = match o.view_tag {
                        Some(view_tag) => {
                            let tagged_key = TaggedKey {
                                key: HexBytes::<32>(o.key.0),
                                view_tag: HexBytes::<1>([view_tag]),
                            };

                            Target::TaggedKey { tagged_key }
                        }
                        None => Target::Key {
                            key: HexBytes::<32>(o.key.0),
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

            TransactionPrefix {
                version,
                unlock_time,
                vin,
                vout,
                extra: prefix.extra,
            }
        }

        #[expect(unused_variables, reason = "TODO: finish impl")]
        match tx {
            transaction::Transaction::V1 { prefix, signatures } => Self::V1 {
                prefix: map_prefix(prefix, 1),
                signatures: signatures
                    .into_iter()
                    .map(|sig| {
                        // TODO: `RingSignature` needs to expose the
                        // inner `Signature` struct as a byte array.
                        let sig_to_64_bytes = |sig| -> HexBytes<64> { todo!() };
                        sig_to_64_bytes(sig)
                    })
                    .collect(),
            },
            transaction::Transaction::V2 { prefix, proofs } => {
                let prefix = map_prefix(prefix, 2);

                let Some(proofs) = proofs else {
                    return Self::V2 {
                        prefix,
                        rct_signatures: RctSignatures::Coinbase { r#type: 0 },
                        rctsig_prunable: None,
                    };
                };

                let r#type = match proofs.rct_type() {
                    ringct::RctType::AggregateMlsagBorromean => 1,
                    ringct::RctType::MlsagBorromean => 2,
                    ringct::RctType::MlsagBulletproofs => 3,
                    ringct::RctType::MlsagBulletproofsCompactAmount => 4,
                    ringct::RctType::ClsagBulletproof => 5,
                    ringct::RctType::ClsagBulletproofPlus => 6,
                };

                let txnFee = proofs.base.fee;

                let ecdhInfo = proofs
                    .base
                    .encrypted_amounts
                    .into_iter()
                    .map(EcdhInfo::from)
                    .collect();

                let outPk = proofs
                    .base
                    .commitments
                    .into_iter()
                    .map(|point| HexBytes::<32>(point.compress().0))
                    .collect();

                let rct_signatures = RctSignatures::NonCoinbase {
                    r#type,
                    txnFee,
                    ecdhInfo,
                    outPk,
                };

                let rctsig_prunable = Some(RctSigPrunable::from(proofs.prunable));

                Self::V2 {
                    prefix,
                    rct_signatures,
                    rctsig_prunable,
                }
            }
        }
    }
}

/// [`Transaction::V2::rct_signatures`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum RctSignatures {
    NonCoinbase {
        r#type: u8,
        txnFee: u64,
        ecdhInfo: Vec<EcdhInfo>,
        outPk: Vec<HexBytes<32>>,
    },
    Coinbase {
        r#type: u8,
    },
}

/// [`Transaction::V2::rctsig_prunable`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum RctSigPrunable {
    /// - [`ringct::RctPrunable::AggregateMlsagBorromean`]
    /// - [`ringct::RctPrunable::MlsagBorromean`]
    MlsagBorromean {
        rangeSigs: Vec<RangeSignature>,
        MGs: Vec<Mg>,
    },

    /// - [`ringct::RctPrunable::MlsagBulletproofs`]
    MlsagBulletproofs {
        nbp: u64,
        bp: Vec<Bulletproof>,
        MGs: Vec<Mg>,
        pseudoOuts: Vec<HexBytes<32>>,
    },

    /// - [`ringct::RctPrunable::Clsag`] with [`ringct::bulletproofs::Bulletproof::Original`]
    ClsagBulletproofs {
        nbp: u64,
        bp: Vec<Bulletproof>,
        CLSAGs: Vec<Clsag>,
        pseudoOuts: Vec<HexBytes<32>>,
    },

    /// - [`ringct::RctPrunable::Clsag`] with [`ringct::bulletproofs::Bulletproof::Plus`]
    ClsagBulletproofsPlus {
        nbp: u64,
        bpp: Vec<BulletproofPlus>,
        CLSAGs: Vec<Clsag>,
        pseudoOuts: Vec<HexBytes<32>>,
    },
}

#[expect(unused_variables, reason = "TODO: finish impl")]
impl From<ringct::RctPrunable> for RctSigPrunable {
    fn from(r: ringct::RctPrunable) -> Self {
        use ringct::RctPrunable as R;

        match r {
            R::AggregateMlsagBorromean { mlsag, borromean } => {
                todo!()
            }
            R::MlsagBorromean { mlsags, borromean } => {
                todo!()
            }
            R::MlsagBulletproofs {
                mlsags,
                pseudo_outs,
                bulletproof,
            } => {
                todo!()
            }
            R::MlsagBulletproofsCompactAmount {
                mlsags,
                pseudo_outs,
                bulletproof,
            } => {
                todo!()
            }
            R::Clsag {
                clsags,
                pseudo_outs,
                bulletproof,
            } => {
                todo!()
            }
        }
    }
}

/// [`RctSigPrunable::MlsagBorromean::rangeSigs`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RangeSignature {
    // These fields are hex but way too big to be
    // using stack arrays to represent them.
    pub asig: String,
    pub Ci: String,
}

/// - [`RctSigPrunable::MlsagBorromean::MGs`]
/// - [`RctSigPrunable::MlsagBulletproofs::MGs`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mg {
    pub ss: Vec<[HexBytes<32>; 2]>,
    pub cc: HexBytes<32>,
}

/// - [`RctSigPrunable::MlsagBulletproofs::bp`]
/// - [`RctSigPrunable::ClsagBulletproofs::bp`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bulletproof {
    pub A: HexBytes<32>,
    pub S: HexBytes<32>,
    pub T1: HexBytes<32>,
    pub T2: HexBytes<32>,
    pub taux: HexBytes<32>,
    pub mu: HexBytes<32>,
    pub L: Vec<HexBytes<32>>,
    pub R: Vec<HexBytes<32>>,
    pub a: HexBytes<32>,
    pub b: HexBytes<32>,
    pub t: HexBytes<32>,
}

/// - [`RctSigPrunable::ClsagBulletproofsPlus::bpp`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BulletproofPlus {
    pub A: HexBytes<32>,
    pub A1: HexBytes<32>,
    pub B: HexBytes<32>,
    pub r1: HexBytes<32>,
    pub s1: HexBytes<32>,
    pub d1: HexBytes<32>,
    pub L: Vec<HexBytes<32>>,
    pub R: Vec<HexBytes<32>>,
}

/// - [`RctSigPrunable::ClsagBulletproofs`]
/// - [`RctSigPrunable::ClsagBulletproofsPlus`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Clsag {
    pub s: Vec<HexBytes<32>>,
    pub c1: HexBytes<32>,
    pub D: HexBytes<32>,
}

/// [`RctSignatures::NonCoinbase::ecdhInfo`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[expect(variant_size_differences)]
pub enum EcdhInfo {
    Original {
        mask: HexBytes<32>,
        amount: HexBytes<32>,
    },
    Compact {
        amount: HexBytes<8>,
    },
}

impl From<ringct::EncryptedAmount> for EcdhInfo {
    fn from(ea: ringct::EncryptedAmount) -> Self {
        match ea {
            ringct::EncryptedAmount::Original { amount, mask } => Self::Original {
                amount: HexBytes::<32>(amount),
                mask: HexBytes::<32>(mask),
            },
            ringct::EncryptedAmount::Compact { amount } => Self::Compact {
                amount: HexBytes::<8>(amount),
            },
        }
    }
}

/// [`TransactionPrefix::vin`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Input {
    pub key: Key,
}

/// [`Input::key`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Key {
    pub amount: u64,
    pub key_offsets: Vec<u64>,
    pub k_image: HexBytes<32>,
}

#[cfg(test)]
mod test {
    use hex_literal::hex;
    use pretty_assertions::assert_eq;

    use super::*;

    #[expect(clippy::needless_pass_by_value)]
    fn test(tx: Transaction, tx_json: &'static str) {
        let json = serde_json::from_str::<Transaction>(tx_json).unwrap();
        assert_eq!(tx, json);
        let string = serde_json::to_string(&json).unwrap();
        assert_eq!(tx_json, &string);
    }

    #[test]
    fn tx_v1() {
        const JSON: &str = r#"{"version":1,"unlock_time":0,"vin":[{"key":{"amount":2865950000,"key_offsets":[0],"k_image":"f1b0eeff62493ea78b2b7e843c278d6d5a7b09adf0cbc83560380d1fe397d6f3"}},{"key":{"amount":6000000000000,"key_offsets":[75146],"k_image":"3d289ab83c06e0948a460e836699a33fe9c300b2448c0f2de0e3b40c13d9bd78"}},{"key":{"amount":3000000000000,"key_offsets":[49742],"k_image":"52a32e6ecadcce76c11262eda8f7265c098b3da1f6e27ae8c9656636faf51ae4"}}],"vout":[{"amount":29220020000,"target":{"key":"f9da453f7cd5248e109de3216208eb9ec8617b0739450405de582f09b7e3fc47"}},{"amount":400000000000,"target":{"key":"c31ce6d52fae900ffab9f30b036bbdea0b9442b589cbe24c2e071ddb8291da14"}},{"amount":400000000000,"target":{"key":"bd570e3805c0198c92f9a24d8f12e9dbe88570196efd176b7f186ade904803f4"}},{"amount":1000000000000,"target":{"key":"84d1ba528dfc2e2ff29b3840fc3ae1c87ae5f750e582b78c4161a6bdb6a4717a"}},{"amount":7000000000000,"target":{"key":"993fd478527fd3e790fd3f5a0d9a3a39bebe72598cc81cb9936e08dea7e5fb0f"}}],"extra":[2,33,0,236,254,1,219,138,20,181,240,174,155,149,49,142,23,185,3,251,47,59,239,236,73,246,142,19,181,27,254,76,248,75,191,1,180,204,225,45,175,103,127,119,53,211,168,192,138,14,121,64,19,218,222,27,66,129,115,185,5,113,142,40,157,70,87,62],"signatures":["318755c67c5d3379b0958a047f5439cf43dd251f64b6314c84b2edbf240d950abbeaad13233700e6b6c59bea178c6fbaa246b8fd84b5caf94d1affd520e6770b","a47e6a65e907e49442828db46475ecdf27f3c472f24688423ac97f0efbd8b90b164ed52c070f7a2a95b95398814b19c0befd14a4aab5520963daf3482604df01","fa6981c969c2a1b9d330a8901d2ef7def7f3ade8d9fba444e18e7e349e286a035ae1729a76e01bbbb3ccd010502af6c77049e3167cf108be69706a8674b0c508"]}"#;

        let tx = Transaction::V1 {
            prefix: TransactionPrefix {
                version: 1,
                unlock_time: 0,
                vin: vec![
                    Input {
                        key: Key {
                            amount: 2865950000,
                            key_offsets: vec![0],
                            k_image: HexBytes::<32>(hex!(
                                "f1b0eeff62493ea78b2b7e843c278d6d5a7b09adf0cbc83560380d1fe397d6f3"
                            )),
                        },
                    },
                    Input {
                        key: Key {
                            amount: 6000000000000,
                            key_offsets: vec![75146],
                            k_image: HexBytes::<32>(hex!(
                                "3d289ab83c06e0948a460e836699a33fe9c300b2448c0f2de0e3b40c13d9bd78"
                            )),
                        },
                    },
                    Input {
                        key: Key {
                            amount: 3000000000000,
                            key_offsets: vec![49742],
                            k_image: HexBytes::<32>(hex!(
                                "52a32e6ecadcce76c11262eda8f7265c098b3da1f6e27ae8c9656636faf51ae4"
                            )),
                        },
                    },
                ],
                vout: vec![
                    Output {
                        amount: 29220020000,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "f9da453f7cd5248e109de3216208eb9ec8617b0739450405de582f09b7e3fc47"
                            )),
                        },
                    },
                    Output {
                        amount: 400000000000,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "c31ce6d52fae900ffab9f30b036bbdea0b9442b589cbe24c2e071ddb8291da14"
                            )),
                        },
                    },
                    Output {
                        amount: 400000000000,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "bd570e3805c0198c92f9a24d8f12e9dbe88570196efd176b7f186ade904803f4"
                            )),
                        },
                    },
                    Output {
                        amount: 1000000000000,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "84d1ba528dfc2e2ff29b3840fc3ae1c87ae5f750e582b78c4161a6bdb6a4717a"
                            )),
                        },
                    },
                    Output {
                        amount: 7000000000000,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "993fd478527fd3e790fd3f5a0d9a3a39bebe72598cc81cb9936e08dea7e5fb0f"
                            )),
                        },
                    },
                ],
                extra: vec![
                    2, 33, 0, 236, 254, 1, 219, 138, 20, 181, 240, 174, 155, 149, 49, 142, 23, 185,
                    3, 251, 47, 59, 239, 236, 73, 246, 142, 19, 181, 27, 254, 76, 248, 75, 191, 1,
                    180, 204, 225, 45, 175, 103, 127, 119, 53, 211, 168, 192, 138, 14, 121, 64, 19,
                    218, 222, 27, 66, 129, 115, 185, 5, 113, 142, 40, 157, 70, 87, 62,
                ],
            },
            signatures: vec![
              HexBytes::<64>(hex!("318755c67c5d3379b0958a047f5439cf43dd251f64b6314c84b2edbf240d950abbeaad13233700e6b6c59bea178c6fbaa246b8fd84b5caf94d1affd520e6770b")),
              HexBytes::<64>(hex!("a47e6a65e907e49442828db46475ecdf27f3c472f24688423ac97f0efbd8b90b164ed52c070f7a2a95b95398814b19c0befd14a4aab5520963daf3482604df01")),
              HexBytes::<64>(hex!("fa6981c969c2a1b9d330a8901d2ef7def7f3ade8d9fba444e18e7e349e286a035ae1729a76e01bbbb3ccd010502af6c77049e3167cf108be69706a8674b0c508"))
            ],
        };

        test(tx, JSON);
    }

    #[test]
    fn tx_rct_3() {
        const JSON: &str = r#"{"version":2,"unlock_time":0,"vin":[{"key":{"amount":0,"key_offsets":[8608351,301575,15985,56460,28593,9238,1709,170,369,1874,681],"k_image":"86e1cc68d3970757c4a265a7c28c3a39fe230851f2d8a14c5916a6aa60dbc892"}}],"vout":[{"amount":0,"target":{"key":"f21fd68e011df2e544a3d33221172baf921a121c85d1a2190c42e81d5dd1830e"}},{"amount":0,"target":{"key":"64a3e54d80a729f69ae04f85db06dd26a96f3b05674f6927337a755a9cdaefeb"}},{"amount":0,"target":{"key":"ad2ccf74d2c99946af10cedc922a87c30a4b1c0d7a13143e71d31cd788b0c171"}},{"amount":0,"target":{"key":"e03d9b552a50734487ed0da40ba977f718c91a782fe85899debfd2c56ea3e551"}},{"amount":0,"target":{"key":"b312d502c1b71a10d9483fb816e17d27d0508b5b74c462e14dca52395a14a155"}}],"extra":[1,224,97,197,75,148,190,193,227,206,86,37,3,184,209,129,160,202,192,210,43,32,138,51,151,70,119,47,146,57,223,154,50,4,5,0,191,65,91,3,171,12,85,167,171,16,104,155,253,200,141,80,208,150,162,213,53,57,197,121,68,106,70,96,188,175,119,160,72,148,223,225,199,34,97,143,5,107,219,86,93,114,31,160,6,17,195,221,157,186,50,144,147,117,99,25,160,173,15,167,20,15,91,127,217,28,255,151,248,119,197,199,201,4,248,90,115,44,13,172,116,229,191,216,187,111,255,104,43,62,207,138,134,126,114,34,99,248,241,243,25,208,2,7,247,134,6,9,213,173,242,95,39,187,214,105,81,94,111,53,212,160,183,216,152,137,123,32,101,253,223,108,59,6,176,24,161,45,42,98,92,200,74,34,7,116,231,53,86,94,40,84,151,129,250],"rct_signatures":{"type":3,"txnFee":86000000,"ecdhInfo":[{"mask":"95f1dcd5076d92d9592f1ad5d2e60e8b353c0048da1833db3d11634a9deff50f","amount":"9386f5401e2577e66dea290aae981a85f75ab81d21cd8060b6a2070c0c3d4209"},{"mask":"9a3015d73ee53f40c4a194c725aa5cea4822b99442ddb94223a52e365e02f70b","amount":"40b471293514f4399782abfe2968f5bb297a77b16b58261df7cffc60b68a5b04"},{"mask":"64b2b70d2e61fd4ac5c6d92f61d85dda1daf948853cc303a3a39baeeece41e08","amount":"b388bdce5bd31493dae245af4dbfc8486d959ef28af4ad1c1338f43dd3bd5a01"},{"mask":"e8d8b9380c446cace527ea1864d69f524b2c6b8eaf08f0f6c36621e73de49d0a","amount":"c74b47b823b7e5f2744e9643e4490f236eb9de006acd7bb8a32cca2f56223b06"},{"mask":"1ec895cc03e6831311a3ab6f86745312feec53de5aef1e1f204a984f082bff0c","amount":"d16c02a92488cd7d5fdf3c461ff8f4f7e75a18644e810ddd55a153e79464af0a"}],"outPk":["ff1a463fcb285d747df43612cc3bc925d4d27bebb08537b177a0dba960f90131","6b62f6ed7338cbf9b2639620400690676fa9d43aca93da4a9dc56d665a72b506","9363af049e5b0530fd77f884e84838efcabebf5fff57e2f00b49148c6415eafc","2fc11a581752a6465d59217e6a5841170de3ba96e27d2744ad280b5242efa9e7","56b6c2ca082d95600584ca429e6723816d4496cbf6f586cf1cfe2e0319224451"]},"rctsig_prunable":{"nbp":1,"bp":[{"A":"4e3433b32bd6d349774eac4ad467e698125d9e0402d216609ff0f4cfc216c50c","S":"7f6d8c127e4138c6153954836b127694786b890a138bae684eb951fb0fbf9be4","T1":"40ee0b2925d7555c17dd38bb69b80e1cfc05aa8b9dc2bd088c1361321a09d4f4","T2":"1488d918c2acdd6ff9e8d5bf82a08599733b9084cdfb0f891c9254742f2ea258","taux":"9b26002cff6e74e3da8ce59cadea4c8a0d93b9d4d94e6764031c21ecbac5f600","mu":"a414b36b00a290c62a443282295f50f381a44197af5db74f518a1b77dd8c120a","L":["d4c3360932332dd3cc407d857c6e759d7c29d10deede96a0649bba89fbdb0e04","33d7311748c6ee1fa92311513a3f32acf0bbcbd1c507e4378db19f108914f6c1","aeedddc3feaa374880a388a992e38027d97c8e0e3728fd536fb2f0a560f5f481","662e94760e3d51cf89a39db06c8f95e08b923ed97e883e9144d15f21e1639011","c07d35cb78309eec01052227df1967c0e0d553f6ca5d592f34bbeebcecdc78a8","9954f3a6c818fd5aed6fd7c94fdaf4f49d2159c47e31b953c3e44e11aa4c9943","a22d2b47f1a051daece177803252b976c33ac5e2a8c487afd89d61f3a08180f0","3ce357034185a6f540d00b2ab8229e64d2d6cad27a2b141d6f6e202307d959ae","5906da535fbd816292692880fe4630e9ed1dd2dc94495a4f7db080e18fd4a8e0"],"R":["0b40204226678fee886140e7c20e809165a58e1355101a2c5bdf7c797811ac21","94a1da201d9e85ad6ac564fe2e6a1fa62873d78e33a5931fd143ed165b360eba","fc458a6c42264f6c8890a386b7a68543a952ecc2b6239138b548c25d6bfa6c68","052da59d062001df5d95d3117deecb9b3175ed59a44aba9b92f84add748c1698","5aa7cf7545d4859a62903b29500449813a231a0c25cdb133a4780a9b0a411cd0","5366ad21b6b33b8f43aecfda087f0aee9cfdc2836e59f7721856058685965b39","960c4764aea3c0dff74c88728514da160bd79712cd50a948bd8b52d9569e69b1","6db5c54be77c08460e4581ee7709c0179108b46a55c3858e0897bd4f12e3e913","ffb4d75cab91763dc3e556fce046413382c84abe24615ada0605a43f8de88309"],"a":"43bf84ef0f596d1d76990c5202261f0963dade1affc1eee92a0508f5ce8d2900","b":"747be0d98f642649d2925a459238ed13f65bd6f12019683d4ede505394341604","t":"8592adba69d884c48e52135909a9738eafae80e590ae245b1a9ca65eea3a8b0f"}],"MGs":[{"ss":[["8a8838d965aa1bb49448c12ea1aabb680b393f5bf02e3b73874aa545cde6dc04","e16bf1d0c4c2639af6bed0c0205181b2a03bc5cdc22207906aac710acdd5170e"],["208d25cad34bcc9c49a5516102990814c75e0bbe2335b601880d9c6ce4fb400a","279a89826548b8b15ea342d892ca6f8bf9e6a5a14077a57edaa4fd676b0b9f0f"],["9edbd1d2082bad9dd9ca98baf82b4d70014dee720c758ed0944a9fb82ae55206","3314001eeec40a2e0ca83f48af1ade8b4139418da49e2c6d95aa3a1d4427de07"],["1837f42c1a4bd0747ed86c1e99bfe058031858c47ff4f066cfcdaf107499bf0f","963bd0ed98a01be7c847b393ad0c2c25c3052148d67126c12b25ec2239373005"],["e41e7dd0430ccbc17f717db7fa1720241ab4de24249c607b9f882143d266ff0e","95c4a4ec2756ec57caacb64f17a7e5306103f030dfb12dd53b42c72e68b6e60b"],["8ecfab987a8697c58f4b183620b2fa0e11972fa666b71c138e067621ab5d1703","2e070ae83ab7f01f91766c2fd6de425dc0f18ae4e34fdcb3ac18db4dfec77a0c"],["187cd1a318666e9f7a9f2f9d4eaf7c662c6162c5bc2be94219992f261f46b90b","97ca174ff4bcf1e5d139bf0ad85577b9c6247f9e4782cd69100e683bf2e3f80b"],["28eb6f60cfa35b52cbf74b7e68ce795ebfa0d3db6f00e69677fc98aef963bf05","6662186aa949465b7b2174d6da077ab8ffdddb710bdab42386e7d8ae20f1890d"],["577c9cf99480b0633121737756bcc7f4887fc7fdf3a9344c84578886e60d1404","2d241b48e63acc39c8c899f7c009fcbc09025ea1211930a338e193d17aed890a"],["7a3f489532743f117999a1b375789cd0863541cae0b8633e8cd4c7dedc740305","500c1033ca2b4b47c39e70a1c563553571e0e25a2e1fa984cb5ba08546bc4907"],["82efb453a98454e07e8f4b367ee0db2f957e6222e720a69354fdf910fe5fe803","1c3204cf63c8ba3ebd817d603a4e5cadfa6a9af5999648eabff7605b5de8b306"]],"cc":"8b579f973b9395a175fb2fc1df7d66511166c606903a3c082b63fa831e833b00"}],"pseudoOuts":["bd6260cafa1afbe44d24cf7c42ac9e2b451424472eb1334b3c042e82196be0d7"]}}"#;

        let tx = Transaction::V2 {
            prefix: TransactionPrefix {
                version: 2,
                unlock_time: 0,
                vin: vec![Input {
                    key: Key {
                        amount: 0,
                        key_offsets: vec![
                            8608351, 301575, 15985, 56460, 28593, 9238, 1709, 170, 369, 1874, 681,
                        ],
                        k_image: HexBytes::<32>(hex!(
                            "86e1cc68d3970757c4a265a7c28c3a39fe230851f2d8a14c5916a6aa60dbc892"
                        )),
                    },
                }],
                vout: vec![
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "f21fd68e011df2e544a3d33221172baf921a121c85d1a2190c42e81d5dd1830e"
                            )),
                        },
                    },
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "64a3e54d80a729f69ae04f85db06dd26a96f3b05674f6927337a755a9cdaefeb"
                            )),
                        },
                    },
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "ad2ccf74d2c99946af10cedc922a87c30a4b1c0d7a13143e71d31cd788b0c171"
                            )),
                        },
                    },
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "e03d9b552a50734487ed0da40ba977f718c91a782fe85899debfd2c56ea3e551"
                            )),
                        },
                    },
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "b312d502c1b71a10d9483fb816e17d27d0508b5b74c462e14dca52395a14a155"
                            )),
                        },
                    },
                ],
                extra: vec![
                    1, 224, 97, 197, 75, 148, 190, 193, 227, 206, 86, 37, 3, 184, 209, 129, 160,
                    202, 192, 210, 43, 32, 138, 51, 151, 70, 119, 47, 146, 57, 223, 154, 50, 4, 5,
                    0, 191, 65, 91, 3, 171, 12, 85, 167, 171, 16, 104, 155, 253, 200, 141, 80, 208,
                    150, 162, 213, 53, 57, 197, 121, 68, 106, 70, 96, 188, 175, 119, 160, 72, 148,
                    223, 225, 199, 34, 97, 143, 5, 107, 219, 86, 93, 114, 31, 160, 6, 17, 195, 221,
                    157, 186, 50, 144, 147, 117, 99, 25, 160, 173, 15, 167, 20, 15, 91, 127, 217,
                    28, 255, 151, 248, 119, 197, 199, 201, 4, 248, 90, 115, 44, 13, 172, 116, 229,
                    191, 216, 187, 111, 255, 104, 43, 62, 207, 138, 134, 126, 114, 34, 99, 248,
                    241, 243, 25, 208, 2, 7, 247, 134, 6, 9, 213, 173, 242, 95, 39, 187, 214, 105,
                    81, 94, 111, 53, 212, 160, 183, 216, 152, 137, 123, 32, 101, 253, 223, 108, 59,
                    6, 176, 24, 161, 45, 42, 98, 92, 200, 74, 34, 7, 116, 231, 53, 86, 94, 40, 84,
                    151, 129, 250,
                ],
            },
            rct_signatures: RctSignatures::NonCoinbase {
                r#type: 3,
                txnFee: 86000000,
                ecdhInfo: vec![
                    EcdhInfo::Original {
                        mask: HexBytes::<32>(hex!(
                            "95f1dcd5076d92d9592f1ad5d2e60e8b353c0048da1833db3d11634a9deff50f"
                        )),
                        amount: HexBytes::<32>(hex!(
                            "9386f5401e2577e66dea290aae981a85f75ab81d21cd8060b6a2070c0c3d4209"
                        )),
                    },
                    EcdhInfo::Original {
                        mask: HexBytes::<32>(hex!(
                            "9a3015d73ee53f40c4a194c725aa5cea4822b99442ddb94223a52e365e02f70b"
                        )),
                        amount: HexBytes::<32>(hex!(
                            "40b471293514f4399782abfe2968f5bb297a77b16b58261df7cffc60b68a5b04"
                        )),
                    },
                    EcdhInfo::Original {
                        mask: HexBytes::<32>(hex!(
                            "64b2b70d2e61fd4ac5c6d92f61d85dda1daf948853cc303a3a39baeeece41e08"
                        )),
                        amount: HexBytes::<32>(hex!(
                            "b388bdce5bd31493dae245af4dbfc8486d959ef28af4ad1c1338f43dd3bd5a01"
                        )),
                    },
                    EcdhInfo::Original {
                        mask: HexBytes::<32>(hex!(
                            "e8d8b9380c446cace527ea1864d69f524b2c6b8eaf08f0f6c36621e73de49d0a"
                        )),
                        amount: HexBytes::<32>(hex!(
                            "c74b47b823b7e5f2744e9643e4490f236eb9de006acd7bb8a32cca2f56223b06"
                        )),
                    },
                    EcdhInfo::Original {
                        mask: HexBytes::<32>(hex!(
                            "1ec895cc03e6831311a3ab6f86745312feec53de5aef1e1f204a984f082bff0c"
                        )),
                        amount: HexBytes::<32>(hex!(
                            "d16c02a92488cd7d5fdf3c461ff8f4f7e75a18644e810ddd55a153e79464af0a"
                        )),
                    },
                ],
                outPk: vec![
                    HexBytes::<32>(hex!(
                        "ff1a463fcb285d747df43612cc3bc925d4d27bebb08537b177a0dba960f90131"
                    )),
                    HexBytes::<32>(hex!(
                        "6b62f6ed7338cbf9b2639620400690676fa9d43aca93da4a9dc56d665a72b506"
                    )),
                    HexBytes::<32>(hex!(
                        "9363af049e5b0530fd77f884e84838efcabebf5fff57e2f00b49148c6415eafc"
                    )),
                    HexBytes::<32>(hex!(
                        "2fc11a581752a6465d59217e6a5841170de3ba96e27d2744ad280b5242efa9e7"
                    )),
                    HexBytes::<32>(hex!(
                        "56b6c2ca082d95600584ca429e6723816d4496cbf6f586cf1cfe2e0319224451"
                    )),
                ],
            },
            rctsig_prunable: Some(RctSigPrunable::MlsagBulletproofs {
                nbp: 1,
                bp: vec![Bulletproof {
                    A: HexBytes::<32>(hex!(
                        "4e3433b32bd6d349774eac4ad467e698125d9e0402d216609ff0f4cfc216c50c"
                    )),
                    S: HexBytes::<32>(hex!(
                        "7f6d8c127e4138c6153954836b127694786b890a138bae684eb951fb0fbf9be4"
                    )),
                    T1: HexBytes::<32>(hex!(
                        "40ee0b2925d7555c17dd38bb69b80e1cfc05aa8b9dc2bd088c1361321a09d4f4"
                    )),
                    T2: HexBytes::<32>(hex!(
                        "1488d918c2acdd6ff9e8d5bf82a08599733b9084cdfb0f891c9254742f2ea258"
                    )),
                    taux: HexBytes::<32>(hex!(
                        "9b26002cff6e74e3da8ce59cadea4c8a0d93b9d4d94e6764031c21ecbac5f600"
                    )),
                    mu: HexBytes::<32>(hex!(
                        "a414b36b00a290c62a443282295f50f381a44197af5db74f518a1b77dd8c120a"
                    )),
                    L: vec![
                        HexBytes::<32>(hex!(
                            "d4c3360932332dd3cc407d857c6e759d7c29d10deede96a0649bba89fbdb0e04"
                        )),
                        HexBytes::<32>(hex!(
                            "33d7311748c6ee1fa92311513a3f32acf0bbcbd1c507e4378db19f108914f6c1"
                        )),
                        HexBytes::<32>(hex!(
                            "aeedddc3feaa374880a388a992e38027d97c8e0e3728fd536fb2f0a560f5f481"
                        )),
                        HexBytes::<32>(hex!(
                            "662e94760e3d51cf89a39db06c8f95e08b923ed97e883e9144d15f21e1639011"
                        )),
                        HexBytes::<32>(hex!(
                            "c07d35cb78309eec01052227df1967c0e0d553f6ca5d592f34bbeebcecdc78a8"
                        )),
                        HexBytes::<32>(hex!(
                            "9954f3a6c818fd5aed6fd7c94fdaf4f49d2159c47e31b953c3e44e11aa4c9943"
                        )),
                        HexBytes::<32>(hex!(
                            "a22d2b47f1a051daece177803252b976c33ac5e2a8c487afd89d61f3a08180f0"
                        )),
                        HexBytes::<32>(hex!(
                            "3ce357034185a6f540d00b2ab8229e64d2d6cad27a2b141d6f6e202307d959ae"
                        )),
                        HexBytes::<32>(hex!(
                            "5906da535fbd816292692880fe4630e9ed1dd2dc94495a4f7db080e18fd4a8e0"
                        )),
                    ],
                    R: vec![
                        HexBytes::<32>(hex!(
                            "0b40204226678fee886140e7c20e809165a58e1355101a2c5bdf7c797811ac21"
                        )),
                        HexBytes::<32>(hex!(
                            "94a1da201d9e85ad6ac564fe2e6a1fa62873d78e33a5931fd143ed165b360eba"
                        )),
                        HexBytes::<32>(hex!(
                            "fc458a6c42264f6c8890a386b7a68543a952ecc2b6239138b548c25d6bfa6c68"
                        )),
                        HexBytes::<32>(hex!(
                            "052da59d062001df5d95d3117deecb9b3175ed59a44aba9b92f84add748c1698"
                        )),
                        HexBytes::<32>(hex!(
                            "5aa7cf7545d4859a62903b29500449813a231a0c25cdb133a4780a9b0a411cd0"
                        )),
                        HexBytes::<32>(hex!(
                            "5366ad21b6b33b8f43aecfda087f0aee9cfdc2836e59f7721856058685965b39"
                        )),
                        HexBytes::<32>(hex!(
                            "960c4764aea3c0dff74c88728514da160bd79712cd50a948bd8b52d9569e69b1"
                        )),
                        HexBytes::<32>(hex!(
                            "6db5c54be77c08460e4581ee7709c0179108b46a55c3858e0897bd4f12e3e913"
                        )),
                        HexBytes::<32>(hex!(
                            "ffb4d75cab91763dc3e556fce046413382c84abe24615ada0605a43f8de88309"
                        )),
                    ],
                    a: HexBytes::<32>(hex!(
                        "43bf84ef0f596d1d76990c5202261f0963dade1affc1eee92a0508f5ce8d2900"
                    )),
                    b: HexBytes::<32>(hex!(
                        "747be0d98f642649d2925a459238ed13f65bd6f12019683d4ede505394341604"
                    )),
                    t: HexBytes::<32>(hex!(
                        "8592adba69d884c48e52135909a9738eafae80e590ae245b1a9ca65eea3a8b0f"
                    )),
                }],
                MGs: vec![Mg {
                    ss: vec![
                        [
                            HexBytes::<32>(hex!(
                                "8a8838d965aa1bb49448c12ea1aabb680b393f5bf02e3b73874aa545cde6dc04"
                            )),
                            HexBytes::<32>(hex!(
                                "e16bf1d0c4c2639af6bed0c0205181b2a03bc5cdc22207906aac710acdd5170e"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "208d25cad34bcc9c49a5516102990814c75e0bbe2335b601880d9c6ce4fb400a"
                            )),
                            HexBytes::<32>(hex!(
                                "279a89826548b8b15ea342d892ca6f8bf9e6a5a14077a57edaa4fd676b0b9f0f"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "9edbd1d2082bad9dd9ca98baf82b4d70014dee720c758ed0944a9fb82ae55206"
                            )),
                            HexBytes::<32>(hex!(
                                "3314001eeec40a2e0ca83f48af1ade8b4139418da49e2c6d95aa3a1d4427de07"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "1837f42c1a4bd0747ed86c1e99bfe058031858c47ff4f066cfcdaf107499bf0f"
                            )),
                            HexBytes::<32>(hex!(
                                "963bd0ed98a01be7c847b393ad0c2c25c3052148d67126c12b25ec2239373005"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "e41e7dd0430ccbc17f717db7fa1720241ab4de24249c607b9f882143d266ff0e"
                            )),
                            HexBytes::<32>(hex!(
                                "95c4a4ec2756ec57caacb64f17a7e5306103f030dfb12dd53b42c72e68b6e60b"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "8ecfab987a8697c58f4b183620b2fa0e11972fa666b71c138e067621ab5d1703"
                            )),
                            HexBytes::<32>(hex!(
                                "2e070ae83ab7f01f91766c2fd6de425dc0f18ae4e34fdcb3ac18db4dfec77a0c"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "187cd1a318666e9f7a9f2f9d4eaf7c662c6162c5bc2be94219992f261f46b90b"
                            )),
                            HexBytes::<32>(hex!(
                                "97ca174ff4bcf1e5d139bf0ad85577b9c6247f9e4782cd69100e683bf2e3f80b"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "28eb6f60cfa35b52cbf74b7e68ce795ebfa0d3db6f00e69677fc98aef963bf05"
                            )),
                            HexBytes::<32>(hex!(
                                "6662186aa949465b7b2174d6da077ab8ffdddb710bdab42386e7d8ae20f1890d"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "577c9cf99480b0633121737756bcc7f4887fc7fdf3a9344c84578886e60d1404"
                            )),
                            HexBytes::<32>(hex!(
                                "2d241b48e63acc39c8c899f7c009fcbc09025ea1211930a338e193d17aed890a"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "7a3f489532743f117999a1b375789cd0863541cae0b8633e8cd4c7dedc740305"
                            )),
                            HexBytes::<32>(hex!(
                                "500c1033ca2b4b47c39e70a1c563553571e0e25a2e1fa984cb5ba08546bc4907"
                            )),
                        ],
                        [
                            HexBytes::<32>(hex!(
                                "82efb453a98454e07e8f4b367ee0db2f957e6222e720a69354fdf910fe5fe803"
                            )),
                            HexBytes::<32>(hex!(
                                "1c3204cf63c8ba3ebd817d603a4e5cadfa6a9af5999648eabff7605b5de8b306"
                            )),
                        ],
                    ],
                    cc: HexBytes::<32>(hex!(
                        "8b579f973b9395a175fb2fc1df7d66511166c606903a3c082b63fa831e833b00"
                    )),
                }],
                pseudoOuts: vec![HexBytes::<32>(hex!(
                    "bd6260cafa1afbe44d24cf7c42ac9e2b451424472eb1334b3c042e82196be0d7"
                ))],
            }),
        };

        test(tx, JSON);
    }

    #[test]
    fn tx_rct_5() {
        const JSON: &str = r#"{"version":2,"unlock_time":0,"vin":[{"key":{"amount":0,"key_offsets":[21656060,186727,69935,9151,6868,5611,37323,11548,1080,2867,1193],"k_image":"2969fedfe8eff9fe1aa29c73ea55e8a9080c25dc565d2703e4d2776991a158bd"}}],"vout":[{"amount":0,"target":{"key":"4a46669165c842dcc4529cb0ca7e34b47073a96d5b29862c9f052a6113ac4db6"}},{"amount":0,"target":{"key":"264b1dcf7eebde1f4eb9ec87eca25dd963d7281ab5efaa5cfa994a4265fd9b4c"}}],"extra":[1,137,144,107,99,61,229,55,205,33,49,82,78,22,98,81,68,252,30,53,33,0,208,38,36,247,66,155,149,65,35,254,12,2,9,1,151,243,81,31,198,0,170,41],"rct_signatures":{"type":5,"txnFee":13210000,"ecdhInfo":[{"amount":"5db75ce558a47531"},{"amount":"0499d909aafd0109"}],"outPk":["70cbcd5105fcb33f29c8f58b7515f98cfdfcbc425239f65eac3804fbee069995","0aba72c6895d733b7cf59d2cf9c4cd7c82dedf23f9424148d63f138291e6b142"]},"rctsig_prunable":{"nbp":1,"bp":[{"A":"86765eb65aac879a755822a69a54dbf69d2d3495439eff917dc01667b72d30f8","S":"1a1e62a9ca8142cafdd8a8b74517d17f2e883d3495b7722e27750fa3fed44d84","T1":"a6513e0191d0561c16f06dda675e9d21a6f7a074dbf1af498530711a4c0a3b8e","T2":"47a1197d264c8becf36fe2e45bebbe9ff86ab7c141dd99db218ba691a412190b","taux":"cc5aa963d74e48c673f5079b0968060db5c408d8ef50ca8cba9fc58f5e11ff06","mu":"869813119eb1c88103d3b396bb1ee604df3c2ecfd7fab9a70da41f9cb95b2309","L":["34d1b4db37ad7d522d273c134a80d08eb6a22c1e009d3ab7db950090d35accdf","e7b41adc55ec0887b1a982f25c11d50a6191aa0e3de7f92ba944b0967b7b0cd5","343b5ad8c7abe7753ddba2fadb3cef36de91a2757167c102c4bb95c3e6778028","c132bb4bab3e60b86637ce2a3a563ecf92635b4a972083adacf6ede475467eb6","3303f34042776e60631352f687a4508b6e0e96ba58e05da825649c0b342527a8","c927d1a85fab1d83e1d3312e4f136e58f13853e529e3d2738d69e7885713a402","8a440a513f9e71d1a1a6357954b9a90123da3cfde7ed50b9cb389f6963090e49"],"R":["60cec37d53635e0f7cfddf7ab7bd4fc092ac69444aa8ebe1029cdac3505e028d","4b4c26bae4ee65f66246f45a83d8f2b4aca580d3ec53bfb62ed0d33e3e80ea60","f1e6aa90b3ae9e72ce487c1728f73a550b5dc41d971a85a90785b922760b0dcd","66e773ab75baa86936bd6653102be3518470f98b1357abb3251da54f273b0e40","792e4c055a4483088698a828e6b35447a4f890ad590d9e37709e53b7a8d63d0e","f6a43739cc3895d297c42179c9cacc31715f675b332a269f7fdf7c3c645f47c3","483a9954d40d1a9ce9082a7e10b8c06fd6e5e925d87dea0d04c4b05c6e74eda7"],"a":"65b1099198e6b20e5d810147bb0f9b4de297da33fb8ffbde3721a600b91ab504","b":"40280b8a652159007d7e36d2a3d4872ae3b7e25617f486a8eeca993d1146c002","t":"aa7d0c7b795de8736e1881fe4b9457cca1e370352c9a2f791d8902292d67de0d"}],"CLSAGs":[{"s":["27c6ca7f8cbdb7d8e6a1e0d3cc0805e48a809b827ccb70a9b297b7e9fd506f04","25212da093e8a866fe693e31022f8f37547cb38643f0a8373ad20032c0d0970a","c41751c335a147960f4daf5d4f18544eab8661e4509e1def78e3c2a08800ab0e","7a82c4e2e693ad5cf288b369ed647847e2b3ada1faab0727331aebce7e026507","690685c5ecab57799fed9067c88c172c466f1ca2ce6768900af0d7d46d474f0a","1891173b4f269dbeb1e13eecd8deecf3ee9bb864476b85a5639257cf6e9f8402","737980e8606d2da628368934c5c05fd2b6c2d43a2b56c5c6c2163b70c0836b06","274a23f3b8baabb020c4e5315174d12049409cae36af0016a0993cdf97957809","de2f2b04ac951975fda136268e60126a6ca53e7cd6cbbff0c9515256d5a1c50f","d5747b07bc733144c8ef9574213731a30d1239596467e25b6aac4427647b1d0c","5fd4c201cfd87e8fb155c1975e02c06c8de1ab49c84c7948e429798a90d52101"],"c1":"0e118c43701bf377e13d9693f6783963d1e6e2a7bff9d75640eb9e1684c26205","D":"deb55a8e4de5b9c84b8d94d63988ce04048497f91bdd3e3878a3f9e7c313e01c"}],"pseudoOuts":["48604572eb550295c16f5fe4282131ed4fc5de297611f813b12e752b6b67865f"]}}"#;

        let tx = Transaction::V2 {
            prefix: TransactionPrefix {
                version: 2,
                unlock_time: 0,
                vin: vec![Input {
                    key: Key {
                        amount: 0,
                        key_offsets: vec![
                            21656060, 186727, 69935, 9151, 6868, 5611, 37323, 11548, 1080, 2867,
                            1193,
                        ],
                        k_image: HexBytes::<32>(hex!(
                            "2969fedfe8eff9fe1aa29c73ea55e8a9080c25dc565d2703e4d2776991a158bd"
                        )),
                    },
                }],
                vout: vec![
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "4a46669165c842dcc4529cb0ca7e34b47073a96d5b29862c9f052a6113ac4db6"
                            )),
                        },
                    },
                    Output {
                        amount: 0,
                        target: Target::Key {
                            key: HexBytes::<32>(hex!(
                                "264b1dcf7eebde1f4eb9ec87eca25dd963d7281ab5efaa5cfa994a4265fd9b4c"
                            )),
                        },
                    },
                ],
                extra: vec![
                    1, 137, 144, 107, 99, 61, 229, 55, 205, 33, 49, 82, 78, 22, 98, 81, 68, 252,
                    30, 53, 33, 0, 208, 38, 36, 247, 66, 155, 149, 65, 35, 254, 12, 2, 9, 1, 151,
                    243, 81, 31, 198, 0, 170, 41,
                ],
            },
            rct_signatures: RctSignatures::NonCoinbase {
                r#type: 5,
                txnFee: 13210000,
                ecdhInfo: vec![
                    EcdhInfo::Compact {
                        amount: HexBytes::<8>(hex!("5db75ce558a47531")),
                    },
                    EcdhInfo::Compact {
                        amount: HexBytes::<8>(hex!("0499d909aafd0109")),
                    },
                ],
                outPk: vec![
                    HexBytes::<32>(hex!(
                        "70cbcd5105fcb33f29c8f58b7515f98cfdfcbc425239f65eac3804fbee069995"
                    )),
                    HexBytes::<32>(hex!(
                        "0aba72c6895d733b7cf59d2cf9c4cd7c82dedf23f9424148d63f138291e6b142"
                    )),
                ],
            },
            rctsig_prunable: Some(RctSigPrunable::ClsagBulletproofs {
                nbp: 1,
                bp: vec![Bulletproof {
                    A: HexBytes::<32>(hex!(
                        "86765eb65aac879a755822a69a54dbf69d2d3495439eff917dc01667b72d30f8"
                    )),
                    S: HexBytes::<32>(hex!(
                        "1a1e62a9ca8142cafdd8a8b74517d17f2e883d3495b7722e27750fa3fed44d84"
                    )),
                    T1: HexBytes::<32>(hex!(
                        "a6513e0191d0561c16f06dda675e9d21a6f7a074dbf1af498530711a4c0a3b8e"
                    )),
                    T2: HexBytes::<32>(hex!(
                        "47a1197d264c8becf36fe2e45bebbe9ff86ab7c141dd99db218ba691a412190b"
                    )),
                    taux: HexBytes::<32>(hex!(
                        "cc5aa963d74e48c673f5079b0968060db5c408d8ef50ca8cba9fc58f5e11ff06"
                    )),
                    mu: HexBytes::<32>(hex!(
                        "869813119eb1c88103d3b396bb1ee604df3c2ecfd7fab9a70da41f9cb95b2309"
                    )),
                    L: vec![
                        HexBytes::<32>(hex!(
                            "34d1b4db37ad7d522d273c134a80d08eb6a22c1e009d3ab7db950090d35accdf"
                        )),
                        HexBytes::<32>(hex!(
                            "e7b41adc55ec0887b1a982f25c11d50a6191aa0e3de7f92ba944b0967b7b0cd5"
                        )),
                        HexBytes::<32>(hex!(
                            "343b5ad8c7abe7753ddba2fadb3cef36de91a2757167c102c4bb95c3e6778028"
                        )),
                        HexBytes::<32>(hex!(
                            "c132bb4bab3e60b86637ce2a3a563ecf92635b4a972083adacf6ede475467eb6"
                        )),
                        HexBytes::<32>(hex!(
                            "3303f34042776e60631352f687a4508b6e0e96ba58e05da825649c0b342527a8"
                        )),
                        HexBytes::<32>(hex!(
                            "c927d1a85fab1d83e1d3312e4f136e58f13853e529e3d2738d69e7885713a402"
                        )),
                        HexBytes::<32>(hex!(
                            "8a440a513f9e71d1a1a6357954b9a90123da3cfde7ed50b9cb389f6963090e49"
                        )),
                    ],
                    R: vec![
                        HexBytes::<32>(hex!(
                            "60cec37d53635e0f7cfddf7ab7bd4fc092ac69444aa8ebe1029cdac3505e028d"
                        )),
                        HexBytes::<32>(hex!(
                            "4b4c26bae4ee65f66246f45a83d8f2b4aca580d3ec53bfb62ed0d33e3e80ea60"
                        )),
                        HexBytes::<32>(hex!(
                            "f1e6aa90b3ae9e72ce487c1728f73a550b5dc41d971a85a90785b922760b0dcd"
                        )),
                        HexBytes::<32>(hex!(
                            "66e773ab75baa86936bd6653102be3518470f98b1357abb3251da54f273b0e40"
                        )),
                        HexBytes::<32>(hex!(
                            "792e4c055a4483088698a828e6b35447a4f890ad590d9e37709e53b7a8d63d0e"
                        )),
                        HexBytes::<32>(hex!(
                            "f6a43739cc3895d297c42179c9cacc31715f675b332a269f7fdf7c3c645f47c3"
                        )),
                        HexBytes::<32>(hex!(
                            "483a9954d40d1a9ce9082a7e10b8c06fd6e5e925d87dea0d04c4b05c6e74eda7"
                        )),
                    ],
                    a: HexBytes::<32>(hex!(
                        "65b1099198e6b20e5d810147bb0f9b4de297da33fb8ffbde3721a600b91ab504"
                    )),
                    b: HexBytes::<32>(hex!(
                        "40280b8a652159007d7e36d2a3d4872ae3b7e25617f486a8eeca993d1146c002"
                    )),
                    t: HexBytes::<32>(hex!(
                        "aa7d0c7b795de8736e1881fe4b9457cca1e370352c9a2f791d8902292d67de0d"
                    )),
                }],
                CLSAGs: vec![Clsag {
                    s: vec![
                        HexBytes::<32>(hex!(
                            "27c6ca7f8cbdb7d8e6a1e0d3cc0805e48a809b827ccb70a9b297b7e9fd506f04"
                        )),
                        HexBytes::<32>(hex!(
                            "25212da093e8a866fe693e31022f8f37547cb38643f0a8373ad20032c0d0970a"
                        )),
                        HexBytes::<32>(hex!(
                            "c41751c335a147960f4daf5d4f18544eab8661e4509e1def78e3c2a08800ab0e"
                        )),
                        HexBytes::<32>(hex!(
                            "7a82c4e2e693ad5cf288b369ed647847e2b3ada1faab0727331aebce7e026507"
                        )),
                        HexBytes::<32>(hex!(
                            "690685c5ecab57799fed9067c88c172c466f1ca2ce6768900af0d7d46d474f0a"
                        )),
                        HexBytes::<32>(hex!(
                            "1891173b4f269dbeb1e13eecd8deecf3ee9bb864476b85a5639257cf6e9f8402"
                        )),
                        HexBytes::<32>(hex!(
                            "737980e8606d2da628368934c5c05fd2b6c2d43a2b56c5c6c2163b70c0836b06"
                        )),
                        HexBytes::<32>(hex!(
                            "274a23f3b8baabb020c4e5315174d12049409cae36af0016a0993cdf97957809"
                        )),
                        HexBytes::<32>(hex!(
                            "de2f2b04ac951975fda136268e60126a6ca53e7cd6cbbff0c9515256d5a1c50f"
                        )),
                        HexBytes::<32>(hex!(
                            "d5747b07bc733144c8ef9574213731a30d1239596467e25b6aac4427647b1d0c"
                        )),
                        HexBytes::<32>(hex!(
                            "5fd4c201cfd87e8fb155c1975e02c06c8de1ab49c84c7948e429798a90d52101"
                        )),
                    ],
                    c1: HexBytes::<32>(hex!(
                        "0e118c43701bf377e13d9693f6783963d1e6e2a7bff9d75640eb9e1684c26205"
                    )),
                    D: HexBytes::<32>(hex!(
                        "deb55a8e4de5b9c84b8d94d63988ce04048497f91bdd3e3878a3f9e7c313e01c"
                    )),
                }],
                pseudoOuts: vec![HexBytes::<32>(hex!(
                    "48604572eb550295c16f5fe4282131ed4fc5de297611f813b12e752b6b67865f"
                ))],
            }),
        };

        test(tx, JSON);
    }

    #[test]
    fn tx_rct_6() {
        const JSON: &str = r#"{"version":2,"unlock_time":0,"vin":[{"key":{"amount":0,"key_offsets":[56619444,517411,383964,1514827,38358,263974,91303,3018,14681,34540,7767,8131,20234,16575,18300,3587],"k_image":"ec1636db12f12cffa66e8e3286d8216ad7900128c996ffcc96196856daf10585"}},{"key":{"amount":0,"key_offsets":[49738606,2766321,6291275,92656,166783,91733,286477,1130,5724,9633,44284,24605,8133,20600,9906,2115],"k_image":"953c1d93684671eb658284061b6f7724f37c68c3bc24732fb81a09f7056426d0"}},{"key":{"amount":0,"key_offsets":[2971790,44215494,8487702,3226387,861,158991,281736,74021,24277,10705,51824,25824,4951,1235,7824,15715],"k_image":"41a34e8637c3974c9a0444f9c45b361775cc178e4d7d8e07e7d4afdc8e591675"}},{"key":{"amount":0,"key_offsets":[57701146,641169,170653,321459,625073,40514,6448,5687,13246,14743,7359,1788,1054,1061,4460,4059],"k_image":"2d57a890ff948dd7f0ba17940b6b76db2c87163322f0bd5aca29462f9224c777"}}],"vout":[{"amount":0,"target":{"tagged_key":{"key":"570482299e724f78b8441d700aa63388a842c7f5dbcbe5fa859c2c0abad96b30","view_tag":"9f"}}},{"amount":0,"target":{"tagged_key":{"key":"50c1a24ef57aeba07beecd8ddbf294e2501d6fa90ad9712829c00b7293eead96","view_tag":"06"}}}],"extra":[1,254,81,251,73,229,142,177,14,82,43,62,58,255,63,24,202,118,195,65,161,185,96,142,214,48,255,145,202,52,3,199,202,2,9,1,53,17,236,142,199,122,102,77],"rct_signatures":{"type":6,"txnFee":71860000,"ecdhInfo":[{"amount":"b0af37c16a8f08a0"},{"amount":"4cc0843dec9af6b4"}],"outPk":["3c51d83c816a0cb8585641a165e866e0215302af9b498db762db27141e673e15","96eba06bfd2781e65e9a1b1506abfd9ae29dc60fcd29007cd6ad94a8abbf1ecc"]},"rctsig_prunable":{"nbp":1,"bpp":[{"A":"28197d8ac07948082b50274fb8d5bea0f81561d02c88981e0d9b5ffd6e5ee169","A1":"efe6eda671d68a894e1b8aff4a1992f85c4269e17196916cfcdf8519cc94d35e","B":"7e374ac72276298148217d99568c3d4e09f2442864b5be228cd2d01328abe2d2","r1":"a2e06c25825774e5a130944c6c3eaa3c8afea2ca7d2c09e024615ff700be240a","s1":"6ee7e6624941d1e9ea18024f39a9a694ac798fb69084e10bf982d6a58d416c0a","d1":"d30bea1ffb8e79d0fe37d60c55f6e654d1ca388b102b29a6b28c48c2c617b70f","L":["cf6e067c87b9161c424620e83645f13284e64c803b9d7af729d0550d475d2199","159a03db0d038f6691816d9c31b52a325ad13941222ce1791a0285ca0cf0169d","f0276445ea2ec07957fa58675c89aec4dab57d163290e95845dccd484c3e1790","40c19df50385f55b4d53fc101c8eef7d411b76c8b94eadbf464d1401d171ea0a","6b9a8da4219da8f3e944351825eaf66e99ea954ed0e3b4eed0782379f8fd5509","567d12ccd952444055c9a595024f1229a8e0d3ad816f6fd28a448f021603bcc1","44616a4203c430653b12e5a2504e79ea390719a1d6a9557eeb55067ba7efc9d3"],"R":["a7dd6caebda761e8c2ca07e65f9f5b5868777bdc9a4af810d55c691ee62922aa","e8db14727596359e8b2e52bd55ceea81e102028d979f79d464d8afe3fd183de3","0f808f768cec8fe9f5e41d5925eb5c4955a2c16f650ba26e8cf7be2246b4e559","4931dd8eb664e60d86ff259b0e8af24329aefd550170683c324bf8e68ca97169","ce05c6ddb73f36dcd5d94cd6a92220c5c42df15f649d4029c9046fb8a8bf4003","ae2d51efb12a81062c7a6c9d2c4cdb4b6d7561f76e6f8aa554d98717716b8dda","ab4a29f9efa41472ae9dfb13d80c929d904a2fbc72a9d2bce063b19baf3bbdbe"]}],"CLSAGs":[{"s":["fa3c832924a4716bac410b9708ac11ed35d9cb01f3e0c1d3958e77791f9ce905","6b4dfe306de3f55c5507d802347f4c94ae55e0db4f3bf25e1af3ba1ecd993e0d","71c7c612a3dd9d123609df658aaff494787b5cabb5624d5c5d519120f29f5407","d72c30a667f22dbc5bbc8479a4e70094bff1980eb962f3f5ce43954da9a5b009","869470794715faa72ec2cbbb78743448f9dc5bb6383ac2030484adbb760e7a09","6247f181b491a4da82cadbca6272b58365e9160030ed92a1ac5641f9d4163b06","9269814384a16ff2bd297fbce5a614ed67529551ba0c21a26abdaff55c96870a","b10aeaac7f08f1782a2eb4094864f26fcb6c43559b7610ccd7809b90b1c4f003","f38ce2ac13fcdee7be79d0bd98bc17f3df4b1c266a45e1fede7582b12e3a3c0d","1b9f3aee12c9fd4e5aae9cf64bd65f0ad20dbc779f472db0bd338229638a6401","a04b7e6791b7ea10af2a8b0ff2dbfe63fb6036beed0bd09e9319d4728e33130b","a0cd570e0cb80e0fc111468add13b0fc0d8eb4df6942ce3caafedb6c9eee0f07","14b38cbfb7012d1c96a25ea5dcb9bfdfb1a92ffe727dd7a1cb332a9bd630d10f","5f9be3bc2f667e41baaad111e34ac14eefa493b565c4be4ab6eeab286903870b","549bc3275bafd26ab4b89ba14b43976dd317d8d344e37ccbd5a20351a084e005","a93847d26171a9194cfa5a94d7f40576b2e808b4bde927e3398bb0a6e9ad0f0e"],"c1":"794f4e50841235043b39fbcb5b50babf5c4b98339fec9538c2538644ac104f01","D":"6d50f7b691c0bc174aa11d9953b3452b0f249aa4e7edd560ff1e5772f592a618"},{"s":["e8140f6e69981917d78656f0b1572ff748771f3106f6134cca77ae83bc2ff201","7970c1856b630f213e52c825c507f331145c55104611a908c91998dcc76dd40f","8b6899f8eef5bb4c0c830fbb53e34b6089215e0c18b174648fe482675eb0740e","8ff4173d836bddc7fd430b0e2cd7d377f9a81025ebdee6030c19114b7964dc05","8f14171c429fbf9bd4aa5fe67d180e99a6092f8a7e27a97e6fd85c02613a0209","9208e8cc2fd66d6c154202c42bde24f14917b79ccc1b2f31d54880fa14c51202","11da8c69a612d2090b375efb61f9be10a16d6ac3d95e8777cb4d3b0cce028509","f0b097956d07aaf27a4d19463b63bed427b4945321f203be856a1c45e058ed0e","0ad2af34567c40ea4166cd49c368a9ac3bac132c2507f444657de8152465ff0c","ded4f3f699c692d01999b858cb752bb6691418393543fa0b1805115847be8f04","6ef1fa94a6448488fdc4fdc629813940f78706f9045f48b8d69ce717510b7b0e","fbc95294de167bb8a431ff2eacec66637a49927eb15bb443f0ec7c909e301a06","03eec8ccae4fd9942376e3df671ed3de9743b03331ee518a69e0f8fb09644e0e","861c4a794793dd3eaedd1770263124530035378121bde995b27cbf50bfeb0d08","043d02997ff017b110590482dba8a69d2a97092ef7640b8cba5d4271ffc67e04","23f12cabd4d7d69a1c6c6cb960b82a427a0ad2df276786312e2b1c60cb17de06"],"c1":"c0f8e42ef1b220d60fa76532b84dd5def896a25b62eec08691ca438c6abcc40d","D":"9d0844d4ac7c5988e336514ba17841a3bd1aefd5fa024b692ccd6ef6e9364e34"},{"s":["bf3a319fd83070d2c97049e1e2d4636f498a5c0547e5e0144f9eb58247f9a50d","70b626b51f5e934fad8f6df0db79c3d2af88b6a93e3fcf872d64e89d1909b10b","71b200df8b8c5132ba533f1f18924114d451d2b9cca454ea36b7e4b559962307","99cc6995a942ad4e9f993d980a077113d46da70f003539190c5bb9ffb4f6310f","4dac904bc896e0f8690353924bc98f0baf2d3a2e39da272fd34444664eede404","158c1087ae06422bd71a0b59ff7e8f2caa6bbc651b4d0e105551bf91a51f2002","e4d119f8c6d39a02b06aca1627078c37b962463d733a4b25d3b6410bdaad6f0f","16d5e70dc9bd9f8e9d8d74d75db0bf3a144952d7eaab3abc78ce7c66cb958d06","3a0ee94b516a8596bd718ffd87efb76e10b61904033fd0225543680064c5120e","354d44ea658710784c4b3389d4048399302e4d7bfa676ea3de53feba2012e30b","ce00bbc38aa3e018f1231972232a076f42d38e6d75dececee6561c6336c4be00","85094c21f620b87e976f42b742449a048eb303597b1ef362c1a44f76f8d9fa08","8e88e960c771bdd2b3df0e0fddbc0cd0a692807d8432c54d6b6ad2114007d10f","976274603af385a4079a970a5ddba77a01ac7411e9b2303e76207b288830a107","a7f760605b4dffb5b76943e8097b11fb4f2db2fea6354cffc2b96c21aef7a300","7e378e64b7a3ece77d88d966e386e939f56976109ad395b4712cf916f50b4c01"],"c1":"edecc915049e5ead7e5fe36dd70c558ace09f4d3a0c6216be148a51e3a72e302","D":"197665d3b405f42a2053f9e946483435e75d6c4e801427bfeb66cc58c72e2670"},{"s":["20c7f0d492ecf79f1d29305f4e8387238a5927fe676674fe479c129431841607","b9b98379560d7e22a09fcc72db5b1d05870ffdbded5cf560fcaf5303033f7d04","8fc79c2b767ea73f7f552f48d0603b5ee369cdd9535ca06f03fd11e16f08ea0b","7e2bdb348f8a719ffed9d995a35d83ae93a63abe1090fae68a3d23ae47c64402","aa0f6221cc1454b4dbf84b7f8c6e7b89a1c2a3d0f56a2d6302106e47b6b1b50d","08a9283d8b34426eb7b7547fa8fb1573430b99f1c119f2ff9612e82acee98e03","250d2ac44e26782f293eca3deb70fc5c52cb942166b1efb2f78ec32640e02d06","1bc1bcc3de357a4652c03815e59e14cb13668946366746dca3dad2f4c44c9000","9f8b446e373e3e19871f22b9bc95017d4411e555477afb34114b428c8296470a","e49d0313e969fb8c4e451388309280a96b8e3216fa1e28ab2efab49f38e86f07","0cee07c99293507ad558838f2fa07af1c4ddc86886658c6207c1f25f343afb06","39bd17be3aaaeda4fb8aa8dafcf5748581f7bb8b92b0dfe3add14a8481570003","0048e1ca905806551cd210c40356cc80935a98f63163a087ea89585915e8770d","3c46eea5308dbdff7376d89378998376cb722d08604d6ecb2b3cb795f91dc607","7d13be56b2e858d2fca81b3a6b0312943d33e501b4e09814818edb96fb28aa0c","313a2021350abd25bd79c22ea33fe071367da2e208913d788d50101c90f0e702"],"c1":"9d96220cd0d49340e06b915f7204cd1f68c4c2069389bf4c011b4fa6c24c0d02","D":"91d00727ba3655729ce88981e803967946403e970f0a6749625f59d4e6d7ebc9"}],"pseudoOuts":["a6785a3aca529db1da40944bb1826519d7caaa31f4549e6854cb97e5234d3e8e","f5cda4db5f83f1c1edea0b66461d1848daf01054c24a690e1438add59dc4f206","dff30968b66355b9c7890db508692e9620c999e0025ca9395fa53732e6432606","6b78d37b63714ebe1d09981766c61a07bf0bfbc9fc7f7a8998396aa99d43e0cc"]}}"#;

        let tx = Transaction::V2 {
            prefix: TransactionPrefix {
                version: 2,
                unlock_time: 0,
                vin: vec![
                    Input {
                        key: Key {
                            amount: 0,
                            key_offsets: vec![
                                56619444, 517411, 383964, 1514827, 38358, 263974, 91303, 3018,
                                14681, 34540, 7767, 8131, 20234, 16575, 18300, 3587,
                            ],
                            k_image: HexBytes::<32>(hex!(
                                "ec1636db12f12cffa66e8e3286d8216ad7900128c996ffcc96196856daf10585"
                            )),
                        },
                    },
                    Input {
                        key: Key {
                            amount: 0,
                            key_offsets: vec![
                                49738606, 2766321, 6291275, 92656, 166783, 91733, 286477, 1130,
                                5724, 9633, 44284, 24605, 8133, 20600, 9906, 2115,
                            ],
                            k_image: HexBytes::<32>(hex!(
                                "953c1d93684671eb658284061b6f7724f37c68c3bc24732fb81a09f7056426d0"
                            )),
                        },
                    },
                    Input {
                        key: Key {
                            amount: 0,
                            key_offsets: vec![
                                2971790, 44215494, 8487702, 3226387, 861, 158991, 281736, 74021,
                                24277, 10705, 51824, 25824, 4951, 1235, 7824, 15715,
                            ],
                            k_image: HexBytes::<32>(hex!(
                                "41a34e8637c3974c9a0444f9c45b361775cc178e4d7d8e07e7d4afdc8e591675"
                            )),
                        },
                    },
                    Input {
                        key: Key {
                            amount: 0,
                            key_offsets: vec![
                                57701146, 641169, 170653, 321459, 625073, 40514, 6448, 5687, 13246,
                                14743, 7359, 1788, 1054, 1061, 4460, 4059,
                            ],
                            k_image: HexBytes::<32>(hex!(
                                "2d57a890ff948dd7f0ba17940b6b76db2c87163322f0bd5aca29462f9224c777"
                            )),
                        },
                    },
                ],
                vout: vec![
                    Output {
                        amount: 0,
                        target: Target::TaggedKey {
                            tagged_key: TaggedKey {
                                key: HexBytes::<32>(hex!(
                                "570482299e724f78b8441d700aa63388a842c7f5dbcbe5fa859c2c0abad96b30"
                            )),
                                view_tag: HexBytes::<1>(hex!("9f")),
                            },
                        },
                    },
                    Output {
                        amount: 0,
                        target: Target::TaggedKey {
                            tagged_key: TaggedKey {
                                key: HexBytes::<32>(hex!(
                                "50c1a24ef57aeba07beecd8ddbf294e2501d6fa90ad9712829c00b7293eead96"
                            )),
                                view_tag: HexBytes::<1>(hex!("06")),
                            },
                        },
                    },
                ],
                extra: vec![
                    1, 254, 81, 251, 73, 229, 142, 177, 14, 82, 43, 62, 58, 255, 63, 24, 202, 118,
                    195, 65, 161, 185, 96, 142, 214, 48, 255, 145, 202, 52, 3, 199, 202, 2, 9, 1,
                    53, 17, 236, 142, 199, 122, 102, 77,
                ],
            },
            rct_signatures: RctSignatures::NonCoinbase {
                r#type: 6,
                txnFee: 71860000,
                ecdhInfo: vec![
                    EcdhInfo::Compact {
                        amount: HexBytes::<8>(hex!("b0af37c16a8f08a0")),
                    },
                    EcdhInfo::Compact {
                        amount: HexBytes::<8>(hex!("4cc0843dec9af6b4")),
                    },
                ],
                outPk: vec![
                    HexBytes::<32>(hex!(
                        "3c51d83c816a0cb8585641a165e866e0215302af9b498db762db27141e673e15"
                    )),
                    HexBytes::<32>(hex!(
                        "96eba06bfd2781e65e9a1b1506abfd9ae29dc60fcd29007cd6ad94a8abbf1ecc"
                    )),
                ],
            },
            rctsig_prunable: Some(RctSigPrunable::ClsagBulletproofsPlus {
                nbp: 1,
                bpp: vec![BulletproofPlus {
                    A: HexBytes::<32>(hex!(
                        "28197d8ac07948082b50274fb8d5bea0f81561d02c88981e0d9b5ffd6e5ee169"
                    )),
                    A1: HexBytes::<32>(hex!(
                        "efe6eda671d68a894e1b8aff4a1992f85c4269e17196916cfcdf8519cc94d35e"
                    )),
                    B: HexBytes::<32>(hex!(
                        "7e374ac72276298148217d99568c3d4e09f2442864b5be228cd2d01328abe2d2"
                    )),
                    r1: HexBytes::<32>(hex!(
                        "a2e06c25825774e5a130944c6c3eaa3c8afea2ca7d2c09e024615ff700be240a"
                    )),
                    s1: HexBytes::<32>(hex!(
                        "6ee7e6624941d1e9ea18024f39a9a694ac798fb69084e10bf982d6a58d416c0a"
                    )),
                    d1: HexBytes::<32>(hex!(
                        "d30bea1ffb8e79d0fe37d60c55f6e654d1ca388b102b29a6b28c48c2c617b70f"
                    )),
                    L: vec![
                        HexBytes::<32>(hex!(
                            "cf6e067c87b9161c424620e83645f13284e64c803b9d7af729d0550d475d2199"
                        )),
                        HexBytes::<32>(hex!(
                            "159a03db0d038f6691816d9c31b52a325ad13941222ce1791a0285ca0cf0169d"
                        )),
                        HexBytes::<32>(hex!(
                            "f0276445ea2ec07957fa58675c89aec4dab57d163290e95845dccd484c3e1790"
                        )),
                        HexBytes::<32>(hex!(
                            "40c19df50385f55b4d53fc101c8eef7d411b76c8b94eadbf464d1401d171ea0a"
                        )),
                        HexBytes::<32>(hex!(
                            "6b9a8da4219da8f3e944351825eaf66e99ea954ed0e3b4eed0782379f8fd5509"
                        )),
                        HexBytes::<32>(hex!(
                            "567d12ccd952444055c9a595024f1229a8e0d3ad816f6fd28a448f021603bcc1"
                        )),
                        HexBytes::<32>(hex!(
                            "44616a4203c430653b12e5a2504e79ea390719a1d6a9557eeb55067ba7efc9d3"
                        )),
                    ],
                    R: vec![
                        HexBytes::<32>(hex!(
                            "a7dd6caebda761e8c2ca07e65f9f5b5868777bdc9a4af810d55c691ee62922aa"
                        )),
                        HexBytes::<32>(hex!(
                            "e8db14727596359e8b2e52bd55ceea81e102028d979f79d464d8afe3fd183de3"
                        )),
                        HexBytes::<32>(hex!(
                            "0f808f768cec8fe9f5e41d5925eb5c4955a2c16f650ba26e8cf7be2246b4e559"
                        )),
                        HexBytes::<32>(hex!(
                            "4931dd8eb664e60d86ff259b0e8af24329aefd550170683c324bf8e68ca97169"
                        )),
                        HexBytes::<32>(hex!(
                            "ce05c6ddb73f36dcd5d94cd6a92220c5c42df15f649d4029c9046fb8a8bf4003"
                        )),
                        HexBytes::<32>(hex!(
                            "ae2d51efb12a81062c7a6c9d2c4cdb4b6d7561f76e6f8aa554d98717716b8dda"
                        )),
                        HexBytes::<32>(hex!(
                            "ab4a29f9efa41472ae9dfb13d80c929d904a2fbc72a9d2bce063b19baf3bbdbe"
                        )),
                    ],
                }],
                CLSAGs: vec![
                    Clsag {
                        s: vec![
                            HexBytes::<32>(hex!(
                                "fa3c832924a4716bac410b9708ac11ed35d9cb01f3e0c1d3958e77791f9ce905"
                            )),
                            HexBytes::<32>(hex!(
                                "6b4dfe306de3f55c5507d802347f4c94ae55e0db4f3bf25e1af3ba1ecd993e0d"
                            )),
                            HexBytes::<32>(hex!(
                                "71c7c612a3dd9d123609df658aaff494787b5cabb5624d5c5d519120f29f5407"
                            )),
                            HexBytes::<32>(hex!(
                                "d72c30a667f22dbc5bbc8479a4e70094bff1980eb962f3f5ce43954da9a5b009"
                            )),
                            HexBytes::<32>(hex!(
                                "869470794715faa72ec2cbbb78743448f9dc5bb6383ac2030484adbb760e7a09"
                            )),
                            HexBytes::<32>(hex!(
                                "6247f181b491a4da82cadbca6272b58365e9160030ed92a1ac5641f9d4163b06"
                            )),
                            HexBytes::<32>(hex!(
                                "9269814384a16ff2bd297fbce5a614ed67529551ba0c21a26abdaff55c96870a"
                            )),
                            HexBytes::<32>(hex!(
                                "b10aeaac7f08f1782a2eb4094864f26fcb6c43559b7610ccd7809b90b1c4f003"
                            )),
                            HexBytes::<32>(hex!(
                                "f38ce2ac13fcdee7be79d0bd98bc17f3df4b1c266a45e1fede7582b12e3a3c0d"
                            )),
                            HexBytes::<32>(hex!(
                                "1b9f3aee12c9fd4e5aae9cf64bd65f0ad20dbc779f472db0bd338229638a6401"
                            )),
                            HexBytes::<32>(hex!(
                                "a04b7e6791b7ea10af2a8b0ff2dbfe63fb6036beed0bd09e9319d4728e33130b"
                            )),
                            HexBytes::<32>(hex!(
                                "a0cd570e0cb80e0fc111468add13b0fc0d8eb4df6942ce3caafedb6c9eee0f07"
                            )),
                            HexBytes::<32>(hex!(
                                "14b38cbfb7012d1c96a25ea5dcb9bfdfb1a92ffe727dd7a1cb332a9bd630d10f"
                            )),
                            HexBytes::<32>(hex!(
                                "5f9be3bc2f667e41baaad111e34ac14eefa493b565c4be4ab6eeab286903870b"
                            )),
                            HexBytes::<32>(hex!(
                                "549bc3275bafd26ab4b89ba14b43976dd317d8d344e37ccbd5a20351a084e005"
                            )),
                            HexBytes::<32>(hex!(
                                "a93847d26171a9194cfa5a94d7f40576b2e808b4bde927e3398bb0a6e9ad0f0e"
                            )),
                        ],
                        c1: HexBytes::<32>(hex!(
                            "794f4e50841235043b39fbcb5b50babf5c4b98339fec9538c2538644ac104f01"
                        )),
                        D: HexBytes::<32>(hex!(
                            "6d50f7b691c0bc174aa11d9953b3452b0f249aa4e7edd560ff1e5772f592a618"
                        )),
                    },
                    Clsag {
                        s: vec![
                            HexBytes::<32>(hex!(
                                "e8140f6e69981917d78656f0b1572ff748771f3106f6134cca77ae83bc2ff201"
                            )),
                            HexBytes::<32>(hex!(
                                "7970c1856b630f213e52c825c507f331145c55104611a908c91998dcc76dd40f"
                            )),
                            HexBytes::<32>(hex!(
                                "8b6899f8eef5bb4c0c830fbb53e34b6089215e0c18b174648fe482675eb0740e"
                            )),
                            HexBytes::<32>(hex!(
                                "8ff4173d836bddc7fd430b0e2cd7d377f9a81025ebdee6030c19114b7964dc05"
                            )),
                            HexBytes::<32>(hex!(
                                "8f14171c429fbf9bd4aa5fe67d180e99a6092f8a7e27a97e6fd85c02613a0209"
                            )),
                            HexBytes::<32>(hex!(
                                "9208e8cc2fd66d6c154202c42bde24f14917b79ccc1b2f31d54880fa14c51202"
                            )),
                            HexBytes::<32>(hex!(
                                "11da8c69a612d2090b375efb61f9be10a16d6ac3d95e8777cb4d3b0cce028509"
                            )),
                            HexBytes::<32>(hex!(
                                "f0b097956d07aaf27a4d19463b63bed427b4945321f203be856a1c45e058ed0e"
                            )),
                            HexBytes::<32>(hex!(
                                "0ad2af34567c40ea4166cd49c368a9ac3bac132c2507f444657de8152465ff0c"
                            )),
                            HexBytes::<32>(hex!(
                                "ded4f3f699c692d01999b858cb752bb6691418393543fa0b1805115847be8f04"
                            )),
                            HexBytes::<32>(hex!(
                                "6ef1fa94a6448488fdc4fdc629813940f78706f9045f48b8d69ce717510b7b0e"
                            )),
                            HexBytes::<32>(hex!(
                                "fbc95294de167bb8a431ff2eacec66637a49927eb15bb443f0ec7c909e301a06"
                            )),
                            HexBytes::<32>(hex!(
                                "03eec8ccae4fd9942376e3df671ed3de9743b03331ee518a69e0f8fb09644e0e"
                            )),
                            HexBytes::<32>(hex!(
                                "861c4a794793dd3eaedd1770263124530035378121bde995b27cbf50bfeb0d08"
                            )),
                            HexBytes::<32>(hex!(
                                "043d02997ff017b110590482dba8a69d2a97092ef7640b8cba5d4271ffc67e04"
                            )),
                            HexBytes::<32>(hex!(
                                "23f12cabd4d7d69a1c6c6cb960b82a427a0ad2df276786312e2b1c60cb17de06"
                            )),
                        ],
                        c1: HexBytes::<32>(hex!(
                            "c0f8e42ef1b220d60fa76532b84dd5def896a25b62eec08691ca438c6abcc40d"
                        )),
                        D: HexBytes::<32>(hex!(
                            "9d0844d4ac7c5988e336514ba17841a3bd1aefd5fa024b692ccd6ef6e9364e34"
                        )),
                    },
                    Clsag {
                        s: vec![
                            HexBytes::<32>(hex!(
                                "bf3a319fd83070d2c97049e1e2d4636f498a5c0547e5e0144f9eb58247f9a50d"
                            )),
                            HexBytes::<32>(hex!(
                                "70b626b51f5e934fad8f6df0db79c3d2af88b6a93e3fcf872d64e89d1909b10b"
                            )),
                            HexBytes::<32>(hex!(
                                "71b200df8b8c5132ba533f1f18924114d451d2b9cca454ea36b7e4b559962307"
                            )),
                            HexBytes::<32>(hex!(
                                "99cc6995a942ad4e9f993d980a077113d46da70f003539190c5bb9ffb4f6310f"
                            )),
                            HexBytes::<32>(hex!(
                                "4dac904bc896e0f8690353924bc98f0baf2d3a2e39da272fd34444664eede404"
                            )),
                            HexBytes::<32>(hex!(
                                "158c1087ae06422bd71a0b59ff7e8f2caa6bbc651b4d0e105551bf91a51f2002"
                            )),
                            HexBytes::<32>(hex!(
                                "e4d119f8c6d39a02b06aca1627078c37b962463d733a4b25d3b6410bdaad6f0f"
                            )),
                            HexBytes::<32>(hex!(
                                "16d5e70dc9bd9f8e9d8d74d75db0bf3a144952d7eaab3abc78ce7c66cb958d06"
                            )),
                            HexBytes::<32>(hex!(
                                "3a0ee94b516a8596bd718ffd87efb76e10b61904033fd0225543680064c5120e"
                            )),
                            HexBytes::<32>(hex!(
                                "354d44ea658710784c4b3389d4048399302e4d7bfa676ea3de53feba2012e30b"
                            )),
                            HexBytes::<32>(hex!(
                                "ce00bbc38aa3e018f1231972232a076f42d38e6d75dececee6561c6336c4be00"
                            )),
                            HexBytes::<32>(hex!(
                                "85094c21f620b87e976f42b742449a048eb303597b1ef362c1a44f76f8d9fa08"
                            )),
                            HexBytes::<32>(hex!(
                                "8e88e960c771bdd2b3df0e0fddbc0cd0a692807d8432c54d6b6ad2114007d10f"
                            )),
                            HexBytes::<32>(hex!(
                                "976274603af385a4079a970a5ddba77a01ac7411e9b2303e76207b288830a107"
                            )),
                            HexBytes::<32>(hex!(
                                "a7f760605b4dffb5b76943e8097b11fb4f2db2fea6354cffc2b96c21aef7a300"
                            )),
                            HexBytes::<32>(hex!(
                                "7e378e64b7a3ece77d88d966e386e939f56976109ad395b4712cf916f50b4c01"
                            )),
                        ],
                        c1: HexBytes::<32>(hex!(
                            "edecc915049e5ead7e5fe36dd70c558ace09f4d3a0c6216be148a51e3a72e302"
                        )),
                        D: HexBytes::<32>(hex!(
                            "197665d3b405f42a2053f9e946483435e75d6c4e801427bfeb66cc58c72e2670"
                        )),
                    },
                    Clsag {
                        s: vec![
                            HexBytes::<32>(hex!(
                                "20c7f0d492ecf79f1d29305f4e8387238a5927fe676674fe479c129431841607"
                            )),
                            HexBytes::<32>(hex!(
                                "b9b98379560d7e22a09fcc72db5b1d05870ffdbded5cf560fcaf5303033f7d04"
                            )),
                            HexBytes::<32>(hex!(
                                "8fc79c2b767ea73f7f552f48d0603b5ee369cdd9535ca06f03fd11e16f08ea0b"
                            )),
                            HexBytes::<32>(hex!(
                                "7e2bdb348f8a719ffed9d995a35d83ae93a63abe1090fae68a3d23ae47c64402"
                            )),
                            HexBytes::<32>(hex!(
                                "aa0f6221cc1454b4dbf84b7f8c6e7b89a1c2a3d0f56a2d6302106e47b6b1b50d"
                            )),
                            HexBytes::<32>(hex!(
                                "08a9283d8b34426eb7b7547fa8fb1573430b99f1c119f2ff9612e82acee98e03"
                            )),
                            HexBytes::<32>(hex!(
                                "250d2ac44e26782f293eca3deb70fc5c52cb942166b1efb2f78ec32640e02d06"
                            )),
                            HexBytes::<32>(hex!(
                                "1bc1bcc3de357a4652c03815e59e14cb13668946366746dca3dad2f4c44c9000"
                            )),
                            HexBytes::<32>(hex!(
                                "9f8b446e373e3e19871f22b9bc95017d4411e555477afb34114b428c8296470a"
                            )),
                            HexBytes::<32>(hex!(
                                "e49d0313e969fb8c4e451388309280a96b8e3216fa1e28ab2efab49f38e86f07"
                            )),
                            HexBytes::<32>(hex!(
                                "0cee07c99293507ad558838f2fa07af1c4ddc86886658c6207c1f25f343afb06"
                            )),
                            HexBytes::<32>(hex!(
                                "39bd17be3aaaeda4fb8aa8dafcf5748581f7bb8b92b0dfe3add14a8481570003"
                            )),
                            HexBytes::<32>(hex!(
                                "0048e1ca905806551cd210c40356cc80935a98f63163a087ea89585915e8770d"
                            )),
                            HexBytes::<32>(hex!(
                                "3c46eea5308dbdff7376d89378998376cb722d08604d6ecb2b3cb795f91dc607"
                            )),
                            HexBytes::<32>(hex!(
                                "7d13be56b2e858d2fca81b3a6b0312943d33e501b4e09814818edb96fb28aa0c"
                            )),
                            HexBytes::<32>(hex!(
                                "313a2021350abd25bd79c22ea33fe071367da2e208913d788d50101c90f0e702"
                            )),
                        ],
                        c1: HexBytes::<32>(hex!(
                            "9d96220cd0d49340e06b915f7204cd1f68c4c2069389bf4c011b4fa6c24c0d02"
                        )),
                        D: HexBytes::<32>(hex!(
                            "91d00727ba3655729ce88981e803967946403e970f0a6749625f59d4e6d7ebc9"
                        )),
                    },
                ],
                pseudoOuts: vec![
                    HexBytes::<32>(hex!(
                        "a6785a3aca529db1da40944bb1826519d7caaa31f4549e6854cb97e5234d3e8e"
                    )),
                    HexBytes::<32>(hex!(
                        "f5cda4db5f83f1c1edea0b66461d1848daf01054c24a690e1438add59dc4f206"
                    )),
                    HexBytes::<32>(hex!(
                        "dff30968b66355b9c7890db508692e9620c999e0025ca9395fa53732e6432606"
                    )),
                    HexBytes::<32>(hex!(
                        "6b78d37b63714ebe1d09981766c61a07bf0bfbc9fc7f7a8998396aa99d43e0cc"
                    )),
                ],
            }),
        };

        test(tx, JSON);
    }
}
