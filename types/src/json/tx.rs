cfg_if::cfg_if! {
    if #[cfg(feature = "serde")] {
        use serde::{Serialize, Deserialize};
        // use monero_serai::{block::Block, transaction::Transaction};
    }
}

/// TODO
///
/// `/get_transactions`
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Transaction {
    pub version: u64,
    pub unlock_time: u64,
    pub vin: Vec<TransactionInput>,
    pub vout: Vec<TransactionOutput>,
    pub extra: [u8; 32],
    /// [`None`] on pruned transactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<String>>,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionInput {
    pub key: TransactionInputKey,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionInputKey {
    pub amount: u64,
    pub key_offsets: Vec<u64>,
    pub k_image: String,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionOutput {
    pub amount: u64,
    pub target: TransactionOutputTarget,
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionOutputTarget {
    pub key: String,
}
