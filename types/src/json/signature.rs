cfg_if::cfg_if! {
    if #[cfg(feature = "serde")] {
        use serde::{Serialize, Deserialize};
        // use monero_serai::{block::Block, transaction::Transaction};
    }
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Signature {
    #[serde(rename = "ecdhinfo")]
    pub ecdh_info: (),
    #[serde(rename = "outPk")]
    pub out_pk: (),
    #[serde(rename = "txnFee")]
    pub txn_fee: (),
    pub r#type: (),
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RctSignature {
    #[serde(rename = "ecdhinfo")]
    pub ecdh_info: (),
    #[serde(rename = "outPk")]
    pub out_pk: (),
    #[serde(rename = "txnFee")]
    pub txn_fee: (),
    pub r#type: (),
}
