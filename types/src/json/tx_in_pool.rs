//! TODO

#![expect(non_snake_case, reason = "JSON fields have non snake-case casing")]

cfg_if::cfg_if! {
    if #[cfg(feature = "serde")] {
        use serde::{Serialize, Deserialize};
        // use monero_serai::{block::Block, transaction::Transaction};
    }
}

/// TODO
///
/// `/get_transaction_pool`
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionInPool {
    pub version: u64,
    pub unlock_time: u64,
    pub vin: Vec<Input>,
    pub vout: Vec<Output>,
    pub extra: [u8; 32],
    /// [`None`] on pruned transactions.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub signatures: Option<Vec<String>>,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RctSignatures {
    pub r#type: u8,
    pub txnFee: u64,
    pub ecdhInfo: Vec<EcdhInfo>,
    pub outPk: Vec<String>,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RctSigPrunable {
    pub nbp: u64,
    pub bpp: Vec<Bpp>,
    pub CLSAGs: Vec<Clsag>,
    pub pseudoOuts: Vec<String>,
}

/// TODO
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

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Clsag {
    pub s: Vec<String>,
    pub c1: String,
    pub D: String,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EcdhInfo {
    pub amount: String,
    pub mask: String,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Input {
    pub key: InputKey,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InputKey {
    pub amount: u64,
    pub key_offsets: Vec<u64>,
    pub k_image: String,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Output {
    pub amount: u64,
    pub target: OutputTarget,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OutputTarget {
    pub tagged_key: OutputTaggedKey,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OutputTaggedKey {
    pub key: String,
    pub view_tag: String,
}
