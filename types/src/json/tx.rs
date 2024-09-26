//! JSON transaction types.

#![expect(
    non_snake_case,
    reason = "JSON serialization requires non snake-case casing"
)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::json::output::Output;

/// JSON representation of a non-miner transaction.
///
/// Used in:
/// - [`/get_transactions` -> `txs.as_json`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_transactions)
/// - [`/get_transaction_pool` -> `tx_json`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_transaction_pool)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[serde(untagged)]
pub enum Transaction {
    V1 {
        /// This field is flattened.
        #[serde(flatten)]
        prefix: TransactionPrefix,
        signatures: Vec<String>,
    },
    V2 {
        /// This field is flattened.
        #[serde(flatten)]
        prefix: TransactionPrefix,
        rct_signatures: RctSignatures,
        rctsig_prunable: RctSigPrunable,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionPrefix {
    pub version: u8,
    pub unlock_time: u64,
    pub vin: Vec<Input>,
    pub vout: Vec<Output>,
    pub extra: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RctSignatures {
    pub r#type: u8,
    pub txnFee: u64,
    pub ecdhInfo: Vec<EcdhInfo>,
    pub outPk: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RctSigPrunable {
    pub nbp: u64,
    pub bpp: Vec<Bpp>,
    pub CLSAGs: Vec<Clsag>,
    pub pseudoOuts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bpp {
    pub A: String,
    pub A1: String,
    pub B: String,
    pub r1: String,
    pub s1: String,
    pub d1: String,
    pub L: Vec<String>,
    pub R: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Clsag {
    pub s: Vec<String>,
    pub c1: String,
    pub D: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EcdhInfo {
    pub amount: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Input {
    pub key: Key,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Key {
    pub amount: u64,
    pub key_offsets: Vec<u64>,
    pub k_image: String,
}

#[cfg(test)]
mod test {
    use super::*;

    const TX_V1: &str = r#"{
  "version": 1,
  "unlock_time": 0,
  "vin": [
    {
      "key": {
        "amount": 2865950000,
        "key_offsets": [
          0
        ],
        "k_image": "f1b0eeff62493ea78b2b7e843c278d6d5a7b09adf0cbc83560380d1fe397d6f3"
      }
    },
    {
      "key": {
        "amount": 6000000000000,
        "key_offsets": [
          75146
        ],
        "k_image": "3d289ab83c06e0948a460e836699a33fe9c300b2448c0f2de0e3b40c13d9bd78"
      }
    },
    {
      "key": {
        "amount": 3000000000000,
        "key_offsets": [
          49742
        ],
        "k_image": "52a32e6ecadcce76c11262eda8f7265c098b3da1f6e27ae8c9656636faf51ae4"
      }
    }
  ],
  "vout": [
    {
      "amount": 29220020000,
      "target": {
        "key": "f9da453f7cd5248e109de3216208eb9ec8617b0739450405de582f09b7e3fc47"
      }
    },
    {
      "amount": 400000000000,
      "target": {
        "key": "c31ce6d52fae900ffab9f30b036bbdea0b9442b589cbe24c2e071ddb8291da14"
      }
    },
    {
      "amount": 400000000000,
      "target": {
        "key": "bd570e3805c0198c92f9a24d8f12e9dbe88570196efd176b7f186ade904803f4"
      }
    },
    {
      "amount": 1000000000000,
      "target": {
        "key": "84d1ba528dfc2e2ff29b3840fc3ae1c87ae5f750e582b78c4161a6bdb6a4717a"
      }
    },
    {
      "amount": 7000000000000,
      "target": {
        "key": "993fd478527fd3e790fd3f5a0d9a3a39bebe72598cc81cb9936e08dea7e5fb0f"
      }
    }
  ],
  "extra": [
    2,
    33,
    0,
    236,
    254,
    1,
    219,
    138,
    20,
    181,
    240,
    174,
    155,
    149,
    49,
    142,
    23,
    185,
    3,
    251,
    47,
    59,
    239,
    236,
    73,
    246,
    142,
    19,
    181,
    27,
    254,
    76,
    248,
    75,
    191,
    1,
    180,
    204,
    225,
    45,
    175,
    103,
    127,
    119,
    53,
    211,
    168,
    192,
    138,
    14,
    121,
    64,
    19,
    218,
    222,
    27,
    66,
    129,
    115,
    185,
    5,
    113,
    142,
    40,
    157,
    70,
    87,
    62
  ],
  "signatures": [
    "318755c67c5d3379b0958a047f5439cf43dd251f64b6314c84b2edbf240d950abbeaad13233700e6b6c59bea178c6fbaa246b8fd84b5caf94d1affd520e6770b",
    "a47e6a65e907e49442828db46475ecdf27f3c472f24688423ac97f0efbd8b90b164ed52c070f7a2a95b95398814b19c0befd14a4aab5520963daf3482604df01",
    "fa6981c969c2a1b9d330a8901d2ef7def7f3ade8d9fba444e18e7e349e286a035ae1729a76e01bbbb3ccd010502af6c77049e3167cf108be69706a8674b0c508"
  ]
}"#;

    const TX_V2: &str = r#"{
  "version": 2,
  "unlock_time": 0,
  "vin": [
    {
      "key": {
        "amount": 0,
        "key_offsets": [
          104873455,
          7761876,
          1191168,
          515412,
          443037,
          240409,
          417907,
          229271,
          42976,
          60086,
          36001,
          25181,
          8268,
          552,
          43,
          936
        ],
        "k_image": "c8be63514785a7ba52ef83a3e9ef42dde17d254ba1bb17a931ebf64e3538db94"
      }
    },
    {
      "key": {
        "amount": 0,
        "key_offsets": [
          66962661,
          37107349,
          1971927,
          8462913,
          439617,
          194053,
          298194,
          91260,
          202657,
          9043,
          29583,
          38303,
          33141,
          366,
          297,
          3110
        ],
        "k_image": "a61fbf0ddd5ba1f47b1b02954851a1870429f2f4f0e0034d09a565e110000d36"
      }
    }
  ],
  "vout": [
    {
      "amount": 0,
      "target": {
        "tagged_key": {
          "key": "5c27e37aceee3193785ae42c3f4ac3517aef57e298e04f18ce1f7fff189a6de8",
          "view_tag": "37"
        }
      }
    },
    {
      "amount": 0,
      "target": {
        "tagged_key": {
          "key": "ad431308c15cfe0c9160f47d173cd242e3bac77b5ffdc689cd6c5dfdc48a62b7",
          "view_tag": "77"
        }
      }
    },
    {
      "amount": 0,
      "target": {
        "tagged_key": {
          "key": "59668bf8df28e92139c3a1e4a6827ac343355d01c5d4d5e793d62a88e68834b3",
          "view_tag": "1b"
        }
      }
    },
    {
      "amount": 0,
      "target": {
        "tagged_key": {
          "key": "b4ab7f00d3019b5d95c1b58c1264095fdb9fdf5897a7dc71688077b37b4f9e3e",
          "view_tag": "47"
        }
      }
    },
    {
      "amount": 0,
      "target": {
        "tagged_key": {
          "key": "11000d47d01b1413005d63fdcd3d8a12292c120f3eb9090dd3ea242d88285f28",
          "view_tag": "8b"
        }
      }
    }
  ],
  "extra": [
    1,
    156,
    116,
    120,
    105,
    226,
    90,
    59,
    60,
    96,
    67,
    23,
    159,
    24,
    92,
    50,
    203,
    212,
    51,
    36,
    95,
    185,
    2,
    153,
    129,
    0,
    209,
    64,
    153,
    211,
    117,
    50,
    145
  ],
  "rct_signatures": {
    "type": 6,
    "txnFee": 80000000,
    "ecdhInfo": [
      {
        "amount": "3021a356ad9a2fbf"
      },
      {
        "amount": "9b8f973f7a87fa8f"
      },
      {
        "amount": "3e7cece72e8bfb94"
      },
      {
        "amount": "a85e3a67f8868bce"
      },
      {
        "amount": "2770d589d6a03b0d"
      }
    ],
    "outPk": [
      "d086c6b35ed7372c700cbb12e2a2e1b78f202fc2a181c4cc1dc158a472746da7",
      "ec9f8635bda0326937ff7b8ad757a65260b9ec8eda2f57a4d115281906863b35",
      "4d190dca5d18867db8ee45627538669d0cf61f1337d09366d3a0cf5d0b2209d6",
      "62e032a151d7bbc9f3430ef28afddaa3d8dd279cc9e13e2b9fbe32c48df79ee2",
      "6adea4761b71115c712a230b67154a256f7375e5a1b75ccbf0212726932447b3"
    ]
  },
  "rctsig_prunable": {
    "nbp": 1,
    "bpp": [
      {
        "A": "adb7b046ad4acea47a9e481c97b18833f91e7202e09a6eaf22af8d3cf00772f8",
        "A1": "fc8d90de80c1160fe693e4e34bb52aa3aba673bb90cf47441f5aa619ec91df8b",
        "B": "28f19abd4b40596b09f1f157c131cc7a80c4c2ad8beb0c4cf7eba1ce7109c83b",
        "r1": "9e55a9119e231cd62d845e6869ade0945298c8e16cc773cddd9a0bb8eddfd60d",
        "s1": "a992df2f44f1a30b5998b814fdccd38c4db3fe9ee2ca51f37b3d1a8098bd4004",
        "d1": "61deab0c8d7532c683be8c7acd0ee7d80d5caf2715b51b3921fe21e8efa52000",
        "L": [
          "7cc0dd838450265051bb108163c0aabb5fb5fc0ca4a76151ba19127077d2d732",
          "434cc644088c52f3ccd7f10aacbf4f30212ddd13b2574dce2bf9409f7b34de47",
          "0386b8bdae7b85fc040ced37bd0b4d3abac67d51f1b6367e5b77eafac70947c2",
          "426ab7f906ae1ff89802e7c40c7320a381dcada3ba37e048bf020e779ac15ead",
          "262f99fa8fd56506fde92530d063478e98fec4277f190878b5b918785f9ff352",
          "64402683c59318685153ce72f35489a92be49aea3dfff8e6822db399a83f16ca",
          "33561a93fce948dc55fdba2caf7db28d5dc435e3c673cc0cad315de35254a847",
          "8f2676e0d873f980d08c7f495b2bf635177c6e4adc2e8e1ee06191ac2ec4b9df",
          "74e2cc5bf28cabff4d02f024617a3e306399dd7e9b64c4d851a7501070b7f6da"
        ],
        "R": [
          "cde5e88c8e713caea7a24b43f48565b8da478830ecca0fe64a73480b958012e3",
          "856812e06eca491cd70cc27e1b5cdd2c687f68a7d1e3898a370afbab401f27b2",
          "d8ba4e8f39ddc60f57a3cfbff75d06b060d22c29247458684144898afe89ebaf",
          "e24de13affe388ded61f03e31b5904316e357749c209b0240c9fb56f30beb73e",
          "995a65fd649821ccb2cb7ffce2c18f42bca8f26aed818638eae42a55de93221b",
          "9f872c21a7722a3dcbc086807f643ca5c845f9cc8a9872c45f9002e79d33750c",
          "ca945036b842d34cfdcd0d83593fd51cc3afe957d18ac7d62f60f95316394925",
          "2bf60a8710dd0c7248c721bde67c8de6faa3a3e52cbe311871d4129ce38db0d9",
          "f32facb96c62423485961fdb3a1a46e75c4ef427aa5dad52d82f2180d7943ab2"
        ]
      }
    ],
    "CLSAGs": [
      {
        "s": [
          "95f0a2b851b713a95a9de83506163575d0e70845abac0ce505975c5976d68a04",
          "d0905df02715524276879c719f7589e8e04ca9141156420c31c66e3cd2969e02",
          "3dd6edaa79da483b702692191e4fdc915931cd998dbe06ffd0a0799ec6c07108",
          "9f07ea9f72bad8eb2e7c4f85b1d90c8bd9496f4cf0ff388f97f75b56a751ed0d",
          "d9b9ba843a88eef547e4405eaf7e11b830d02524fbbdfb3b500f1dea057bf406",
          "3799dbac4d9f252c5e4d83bcddb21ef238a94a3b4975eb9ea02414f29726630c",
          "7ca20ca61af99f6e364c9ca976bbcf17cbd3edc388742b64766292c2c1288a04",
          "f8841da616ad3244f4953490a3446106988c5817403d96ba35db6893dfa87802",
          "2eaa6f8d6b6c4f57a187f4d43fc992dc2d782e589f617d150447f78007820509",
          "00c472230404c5a84054d8503c6525cd5328b6569234db9e3a58b0a3aa306f01",
          "53bd321e866bb4ed47b43cac58ec3b3e1c6673783b861abfac3a695cf4019a0f",
          "619134575b20438bfeb161b68561b3071b7d7408180be80528846aa2241bb70f",
          "f47a93fe330625119f2c50c4f719eef781a1433a9f589ce6389f3ff7f6c4c306",
          "cc89c22cd2951b14a57a8e84e3ceb5d8663601be8938f426e51aa543d3e64504",
          "85bb683cc3f7a581efa51b661833a39ac2a97298bff2401dd9e683fd6c07f307",
          "1117b4db787168112be7537d0b066c5dcf7b2ec8802281e3bc6d38e5dd58050e"
        ],
        "c1": "ee19318f08b297e532f83fb26b510319752d389cdf75419d70dbe83b2f803704",
        "D": "2c80798ca44b918b52ac55aaf2d4dd7f291dfa96caf1564c5a14c55ca3b247fe"
      },
      {
        "s": [
          "17be660e19be72df803bf9edddad0458e3d274db1dcfe6571b54f9b291477009",
          "c76533fdab4b47af480d8051fab3ad0290dd9446bb50b27ff493e8401c63bc08",
          "2e9bd6d0075461a6d2b252274374d8bfeb64c0e05c970a2c873deebd25dfc303",
          "2cb33a3074fdb945a0fc645428031e63b50f6277f3044e21f82bf0b776234d0e",
          "c3fbe42cf1f9459186a74460ea92f15396cf984bde69b9c7498948d015d09806",
          "94de99c87421c181ea56a4bfb69bc8119dc95bc850addef77c920a6be1670d01",
          "1c57a1e7acb3c8560d4ca32778ec1d9d7d27cd75aa8dc979b4c02e5413a32b03",
          "7e78b90201b46317b136e274cb1a401f08f09d286c7dd5235ffa3bfe4159640d",
          "de85c14dea6095844f33fe62d5f206c384c1c865be1ede42eb73223eb671cc0b",
          "5606f93e9231a8acbb4d611a39ea7b2d5ca4077c5155dfd352b04b35c99b980d",
          "78ea32e2c430e93542f5b90d4357664e8d69eeb323864db55037787dd301bb02",
          "102122debdd6fd13e7b2b3b98f782c0c73bd375eed85babceb6b439c0b2a2d02",
          "a0c1ca8b7b9225f3f7afa0d9f8e64d0f780df09d41952ff9b6e089327b98e102",
          "0ffeee417cc3793c890b5c09ae887b91523b4e9f973694131b3f9f9b1affb808",
          "9e39602c499e9635361434003dec05f72da457e006f9baa43c974ccd58919c06",
          "476bbbdde6c0ea86c0df3eedebe6b5c26005d1eea08e25b85e42e763b1fe3303"
        ],
        "c1": "beee3edaa85848bd41d6241920a986a39a2a6d4da2b94871c017d4e7607de301",
        "D": "9a09b5229935adc0be81442f7d630c1db147cc6cce1be0b7066dcc250845b64c"
      }
    ],
    "pseudoOuts": [
      "dcba4eb02c6c0c1bebe48e056feac096e4888ba63c06a15c520b6da0b46f8d38",
      "ab4c413de7e1e7fa9d0d5d96ae826f17648c201701ffc03e6fe69f5cd7bacd60"
    ]
  }
}"#;

    fn test(tx_json: &'static str) {
        let json = serde_json::from_str::<Transaction>(tx_json).unwrap();
        let string = serde_json::to_string_pretty(&json).unwrap();
        assert_eq!(tx_json, &string);
    }

    #[test]
    fn tx_v1() {
        test(TX_V1);
    }

    #[test]
    fn tx_v2() {
        test(TX_V2);
    }
}
