//! ### Types module
//! This module contains definition and implementations of some of the structures stored in the database.
//! Some of these types are just Wrapper for convenience or re-definition of `monero-rs` database type (see Boog900/monero-rs, "db" branch)
//! Since the database do not use dummy keys, these redefined structs are the same as monerod without the prefix data used as a key.
//! All these types implement [`bincode::Encode`] and [`bincode::Decode`]. They can store `monero-rs` types in their field. In this case, these field
//! use the [`Compat<T>`] wrapper.

use crate::encoding::compat::{Compat, ReaderCompat};
use bincode::{enc::write::Writer, Decode, Encode};
use monero::{
    consensus::{encode, Decodable},
    util::ringct::{Key, RctSig, RctSigBase, RctSigPrunable, RctType, Signature},
    Block, Hash, PublicKey, Transaction, TransactionPrefix, TxIn,
};

// ---- BLOCKS ----

#[derive(Clone, Debug, PartialEq)]
/// [`BlockMetadata`] is a struct containing metadata of a block such as  the block's `timestamp`, the `total_coins_generated` at this height, its `weight`, its difficulty (`diff_lo`)
/// and cumulative difficulty (`diff_hi`), the `block_hash`, the cumulative RingCT (`cum_rct`) and its long term weight (`long_term_block_weight`). The monerod's struct equivalent is `mdb_block_info_4`
/// This struct is used in [`crate::table::blockmetadata`] table.
pub struct BlockMetadata {
    /// Block's timestamp (the time at which it started to be mined)
    pub timestamp: u64,
    /// Total monero supply, this block included
    pub total_coins_generated: u64,
    /// Block's weight (sum of all transactions weights)
    pub weight: u64,
    /// Block's cumulative_difficulty. In monerod this field would have been split into two `u64`, since cpp don't support *natively* `uint128_t`/`u128`
    pub cumulative_difficulty: u128,
    /// Block's hash
    pub block_hash: Hash,
    /// Cumulative number of RingCT outputs up to this block
    pub cum_rct: u64,
    /// Block's long term weight
    pub long_term_block_weight: u64,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
/// [`AltBlock`] is a struct contaning an alternative `block` (defining an alternative mainchain) and its metadata (`block_height`, `cumulative_weight`,
/// `cumulative_difficulty_low`, `cumulative_difficulty_high`, `already_generated_coins`).
/// This struct is used in [`crate::table::altblock`] table.
pub struct AltBlock {
    /// Alternative block's height.
    pub height: u64,
    /// Cumulative weight median at this block
    pub cumulative_weight: u64,
    /// Cumulative difficulty
    pub cumulative_difficulty: u128,
    /// Total generated coins excluding this block's coinbase reward + fees
    pub already_generated_coins: u64,
    /// Actual block data, with Prefix and Transactions.
    /// It is worth noting that monerod implementation do not contain the block in its struct, but still append it at the end of metadata.
    pub block: Compat<Block>,
}

// ---- TRANSACTIONS ----

#[derive(Clone, Debug, PartialEq)]
/// [`TransactionPruned`] is, as its name suggest, the pruned part of a transaction, which is the Transaction Prefix and its RingCT signatures.
/// This struct is used in the [`crate::table::txsprefix`] table.
pub struct TransactionPruned {
    /// The transaction prefix.
    pub prefix: TransactionPrefix,
    /// The RingCT signatures, will only contain the 'sig' field.
    pub rct_signatures: RctSig,
}

impl bincode::Decode for TransactionPruned {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let mut r = ReaderCompat(decoder.reader());

        // We first decode the TransactionPrefix and get the n° of inputs/outputs
        let prefix: TransactionPrefix = Decodable::consensus_decode(&mut r)
            .map_err(|_| bincode::error::DecodeError::Other("Monero-rs decoding failed"))?;

        let (inputs, outputs) = (prefix.inputs.len(), prefix.outputs.len());

        // Handle the prefix accordingly to its version
        match *prefix.version {
            // First transaction format, Pre-RingCT, so the signatures are None
            1 => Ok(TransactionPruned {
                prefix,
                rct_signatures: RctSig { sig: None, p: None },
            }),
            _ => {
                let mut rct_signatures = RctSig { sig: None, p: None };
                // No inputs so no RingCT
                if inputs == 0 {
                    return Ok(TransactionPruned {
                        prefix,
                        rct_signatures,
                    });
                }
                // Otherwise get the RingCT signatures for the tx inputs
                if let Some(sig) = RctSigBase::consensus_decode(&mut r, inputs, outputs)
                    .map_err(|_| bincode::error::DecodeError::Other("Monero-rs decoding failed"))?
                {
                    rct_signatures = RctSig {
                        sig: Some(sig),
                        p: None,
                    };
                }
                // And we return it
                Ok(TransactionPruned {
                    prefix,
                    rct_signatures,
                })
            }
        }
    }
}

impl bincode::Encode for TransactionPruned {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        let writer = encoder.writer();
        // Encoding the Transaction prefix first
        let buf = monero::consensus::serialize(&self.prefix);
        writer.write(&buf)?;
        match *self.prefix.version {
            1 => {} // First transaction format, Pre-RingCT, so the there is no Rct signatures to add
            _ => {
                if let Some(sig) = &self.rct_signatures.sig {
                    // If there is signatures then we append it at the end
                    let buf = monero::consensus::serialize(sig);
                    writer.write(&buf)?;
                }
            }
        }
        Ok(())
    }
}

impl TransactionPruned {
    /// Turns a pruned transaction to a normal transaction with the missing pruned data
    pub fn into_transaction(self, prunable: &[u8]) -> Result<Transaction, encode::Error> {
        let mut r = std::io::Cursor::new(prunable);
        match *self.prefix.version {
            // Pre-RingCT transactions
            1 => {
                let signatures: Result<Vec<Vec<Signature>>, encode::Error> = self
                    .prefix
                    .inputs
                    .iter()
                    .filter_map(|input| match input {
                        TxIn::ToKey { key_offsets, .. } => {
                            let sigs: Result<Vec<Signature>, encode::Error> = key_offsets
                                .iter()
                                .map(|_| Decodable::consensus_decode(&mut r))
                                .collect();
                            Some(sigs)
                        }
                        _ => None,
                    })
                    .collect();
                Ok(Transaction {
                    prefix: self.prefix,
                    signatures: signatures?,
                    rct_signatures: RctSig { sig: None, p: None },
                })
            }
            // Post-RingCT Transactions
            _ => {
                let signatures = Vec::new();
                let mut rct_signatures = RctSig { sig: None, p: None };
                if self.prefix.inputs.is_empty() {
                    return Ok(Transaction {
                        prefix: self.prefix,
                        signatures,
                        rct_signatures: RctSig { sig: None, p: None },
                    });
                }
                if let Some(sig) = self.rct_signatures.sig {
                    let p = {
                        if sig.rct_type != RctType::Null {
                            let mixin_size = if !self.prefix.inputs.is_empty() {
                                match &self.prefix.inputs[0] {
                                    TxIn::ToKey { key_offsets, .. } => key_offsets.len() - 1,
                                    _ => 0,
                                }
                            } else {
                                0
                            };
                            RctSigPrunable::consensus_decode(
                                &mut r,
                                sig.rct_type,
                                self.prefix.inputs.len(),
                                self.prefix.outputs.len(),
                                mixin_size,
                            )?
                        } else {
                            None
                        }
                    };
                    rct_signatures = RctSig { sig: Some(sig), p };
                }
                Ok(Transaction {
                    prefix: self.prefix,
                    signatures,
                    rct_signatures,
                })
            }
        }
    }
}

pub fn get_transaction_prunable_blob<W: std::io::Write + ?Sized>(
    tx: &monero::Transaction,
    w: &mut W,
) -> Result<usize, std::io::Error> {
    let mut len = 0;
    match tx.prefix.version.0 {
        1 => {
            for sig in tx.signatures.iter() {
                for c in sig {
                    len += monero::consensus::encode::Encodable::consensus_encode(c, w)?;
                }
            }
        }
        _ => {
            if let Some(sig) = &tx.rct_signatures.sig {
                if let Some(p) = &tx.rct_signatures.p {
                    len += p.consensus_encode(w, sig.rct_type)?;
                }
            }
        }
    }
    Ok(len)
}

pub fn calculate_prunable_hash(tx: &monero::Transaction, tx_prunable_blob: &[u8]) -> Option<Hash> {
    // V1 transaction don't have prunable hash
    if tx.prefix.version.0 == 1 {
        return None;
    }

    // Checking if it's a miner tx
    if let TxIn::Gen { height: _ } = &tx.prefix.inputs[0] {
        if tx.prefix.inputs.len() == 1 {
            // Returning miner tx's empty hash
            return Some(Hash::from_slice(&[
                0x70, 0xa4, 0x85, 0x5d, 0x04, 0xd8, 0xfa, 0x7b, 0x3b, 0x27, 0x82, 0xca, 0x53, 0xb6,
                0x00, 0xe5, 0xc0, 0x03, 0xc7, 0xdc, 0xb2, 0x7d, 0x7e, 0x92, 0x3c, 0x23, 0xf7, 0x86,
                0x01, 0x46, 0xd2, 0xc5,
            ]));
        }
    };

    // Calculating the hash
    Some(Hash::new(tx_prunable_blob))
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
/// [`TxIndex`] is a struct used in the [`crate::table::txsidentifier`]. It store the `unlock_time` of a transaction, the `height` of the block
/// whose transaction belong to and the Transaction ID (`tx_id`)
pub struct TxIndex {
    /// Transaction ID
    pub tx_id: u64,
    /// The unlock time of this transaction (the height at which it is unlocked, it is not a timestamp)
    pub unlock_time: u64,
    /// The height of the block whose transaction belong to
    pub height: u64, // TODO USELESS already in txs_prunable_tip
}

#[derive(Clone, Debug, Encode, Decode)]
/// [`TxOutputIdx`] is a single-tuple struct used to contain the indexes (amount and amount indices) of the transactions outputs. It is defined for more clarity on its role.
/// This struct is used in [`crate::table::txsoutputs`] table.
pub struct TxOutputIdx(pub Vec<u64>);

// ---- OUTPUTS ----

#[derive(Clone, Debug, Encode, Decode)]
/// [`RctOutkey`] is a struct containing RingCT metadata and an output ID. It is equivalent to the `output_data_t` struct in monerod
/// This struct is used in [`crate::table::outputamounts`]
pub struct RctOutkey {
    // /// amount_index
    //pub amount_index: u64,
    /// The output's ID
    pub output_id: u64,
    /// The output's public key (for spend verification)
    pub pubkey: Compat<PublicKey>,
    /// The output's unlock time (the height at which it is unlocked, it is not a timestamp)
    pub unlock_time: u64,
    /// The height of the block which used this output
    pub height: u64,
    /// The output's amount commitment (for spend verification)
    /// For compatibility with Pre-RingCT outputs, this field is an option. In fact, monerod distinguish between `pre_rct_output_data_t` and `output_data_t` field like that :
    /// ```cpp
    /// // This MUST be identical to output_data_t, without the extra rct data at the end
    /// struct pre_rct_output_data_t
    /// ```
    pub commitment: Option<Compat<Key>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
/// [`OutputMetadata`] is a struct containing Outputs Metadata. It is used in [`crate::table::outputmetadata`]. It is a struct merging the
/// `out_tx_index` tuple with `output_data_t` structure in monerod, without the output ID.
pub struct OutputMetadata {
    pub tx_hash: Compat<Hash>,

    pub local_index: u64,

    pub pubkey: Option<Compat<PublicKey>>,

    pub unlock_time: u64,

    pub height: u64,

    pub commitment: Option<Compat<Key>>,
}

//#[derive(Clone, Debug, Encode, Decode)]
//// [`OutAmountIdx`] is a struct tuple used to contain the two keys used in [`crate::table::outputamounts`] table.
//// In monerod, the database key is the amount while the *cursor key* (the amount index) is the prefix of the actual data being returned.
//// As we prefere to note use cursor with partial data, we prefer to concat these two into a unique key
//pub struct OutAmountIdx(u64,u64);
// MAYBE NOT FINALLY

//#[derive(Clone, Debug, Encode, Decode)]
// /// [`OutTx`] is a struct containing the hash of the transaction whose output belongs to, and the local index of this output.
// /// This struct is used in [`crate::table::outputinherit`].
/*pub struct OutTx {
    /// Output's transaction hash
    pub tx_hash: Compat<Hash>,
    /// Local index of the output
    pub local_index: u64,
}*/

#[cfg(test)]
mod tests {
    use monero::Hash;

    use super::get_transaction_prunable_blob;

    #[test]
    fn calculate_tx_prunable_hash() {
        let prunable_blob: Vec<u8> = vec![
            1, 113, 10, 7, 87, 70, 119, 97, 244, 126, 155, 133, 254, 167, 60, 204, 134, 45, 71, 17,
            87, 21, 252, 8, 218, 233, 219, 192, 84, 181, 196, 74, 213, 2, 246, 222, 66, 45, 152,
            159, 156, 19, 224, 251, 110, 154, 188, 91, 129, 53, 251, 82, 134, 46, 93, 119, 136, 35,
            13, 190, 235, 231, 44, 183, 134, 221, 12, 131, 222, 209, 246, 52, 14, 33, 94, 173, 251,
            233, 18, 154, 91, 72, 229, 180, 43, 35, 152, 130, 38, 82, 56, 179, 36, 168, 54, 41, 62,
            49, 208, 35, 245, 29, 27, 81, 72, 140, 104, 4, 59, 22, 120, 252, 67, 197, 130, 245, 93,
            100, 129, 134, 19, 137, 228, 237, 166, 89, 5, 42, 1, 110, 139, 39, 81, 89, 159, 40,
            239, 211, 251, 108, 82, 68, 125, 182, 75, 152, 129, 74, 73, 208, 215, 15, 63, 3, 106,
            168, 35, 56, 126, 66, 2, 189, 53, 201, 77, 187, 102, 127, 154, 60, 209, 33, 217, 109,
            81, 217, 183, 252, 114, 90, 245, 21, 229, 174, 254, 177, 147, 130, 74, 49, 118, 203,
            14, 7, 118, 221, 81, 181, 78, 97, 224, 76, 160, 134, 73, 206, 204, 199, 201, 30, 201,
            77, 4, 78, 237, 167, 76, 92, 104, 247, 247, 203, 141, 243, 72, 52, 83, 61, 35, 147,
            231, 124, 21, 115, 81, 83, 67, 222, 61, 225, 171, 66, 243, 185, 195, 51, 72, 243, 80,
            104, 4, 166, 54, 199, 235, 193, 175, 4, 242, 42, 146, 170, 90, 212, 101, 208, 113, 58,
            65, 121, 55, 179, 206, 92, 50, 94, 171, 33, 67, 108, 220, 19, 193, 155, 30, 58, 46, 9,
            227, 48, 246, 187, 82, 230, 61, 64, 95, 197, 183, 150, 62, 203, 252, 36, 157, 135, 160,
            120, 189, 52, 94, 186, 93, 5, 36, 120, 160, 62, 254, 178, 101, 11, 228, 63, 128, 249,
            182, 56, 100, 9, 5, 2, 81, 243, 229, 245, 43, 234, 35, 216, 212, 46, 165, 251, 183,
            133, 10, 76, 172, 95, 106, 231, 13, 216, 222, 15, 92, 122, 103, 68, 238, 190, 108, 124,
            138, 62, 255, 243, 22, 209, 2, 138, 45, 178, 101, 240, 18, 186, 71, 239, 137, 191, 134,
            128, 221, 181, 173, 242, 111, 117, 45, 255, 138, 101, 79, 242, 42, 4, 144, 245, 193,
            79, 14, 44, 201, 223, 0, 193, 123, 75, 155, 140, 248, 0, 226, 246, 230, 126, 7, 32,
            107, 173, 193, 206, 184, 11, 33, 148, 104, 32, 79, 149, 71, 68, 150, 6, 47, 90, 231,
            151, 14, 121, 196, 169, 249, 117, 154, 167, 139, 103, 62, 97, 250, 131, 160, 92, 239,
            18, 236, 110, 184, 102, 30, 194, 175, 243, 145, 169, 183, 163, 141, 244, 186, 172, 251,
            3, 78, 165, 33, 12, 2, 136, 180, 178, 83, 117, 0, 184, 170, 255, 69, 131, 123, 8, 212,
            158, 162, 119, 137, 146, 63, 95, 133, 186, 91, 255, 152, 187, 107, 113, 147, 51, 219,
            207, 5, 160, 169, 97, 9, 1, 202, 152, 186, 128, 160, 110, 120, 7, 176, 103, 87, 30,
            137, 240, 67, 55, 79, 147, 223, 45, 177, 210, 101, 225, 22, 25, 129, 111, 101, 21, 213,
            20, 254, 36, 57, 67, 70, 93, 192, 11, 180, 75, 99, 185, 77, 75, 74, 63, 182, 183, 208,
            16, 69, 237, 96, 76, 96, 212, 242, 6, 169, 14, 250, 168, 129, 18, 141, 240, 101, 196,
            96, 120, 88, 90, 51, 77, 12, 133, 212, 192, 107, 131, 238, 34, 237, 93, 157, 108, 13,
            255, 187, 163, 106, 148, 108, 105, 244, 243, 174, 189, 180, 48, 102, 57, 170, 118, 211,
            110, 126, 222, 165, 93, 36, 157, 90, 14, 135, 184, 197, 185, 7, 99, 199, 224, 225, 243,
            212, 116, 149, 137, 186, 16, 196, 73, 23, 11, 248, 248, 67, 167, 149, 154, 64, 76, 218,
            119, 135, 239, 34, 48, 66, 57, 109, 246, 3, 141, 169, 42, 157, 222, 21, 40, 183, 168,
            97, 195, 106, 244, 229, 61, 122, 136, 59, 255, 120, 86, 30, 63, 226, 18, 65, 218, 188,
            195, 217, 85, 12, 211, 221, 188, 27, 8, 98, 103, 211, 213, 217, 65, 82, 229, 145, 80,
            147, 220, 57, 143, 20, 189, 253, 106, 13, 21, 170, 60, 24, 48, 162, 234, 0, 240, 226,
            4, 28, 76, 93, 56, 3, 187, 223, 58, 31, 184, 58, 234, 198, 140, 223, 217, 1, 147, 94,
            218, 199, 154, 121, 137, 44, 229, 0, 1, 10, 133, 250, 140, 64, 150, 89, 64, 112, 178,
            221, 87, 19, 24, 104, 252, 28, 65, 207, 28, 195, 217, 73, 12, 16, 83, 55, 199, 84, 117,
            175, 123, 13, 234, 10, 54, 63, 245, 161, 74, 235, 92, 189, 247, 47, 62, 176, 41, 159,
            40, 250, 116, 63, 33, 193, 78, 72, 29, 215, 9, 191, 233, 243, 87, 14, 195, 7, 89, 101,
            0, 28, 0, 234, 205, 59, 142, 119, 119, 52, 143, 80, 151, 211, 184, 235, 98, 222, 206,
            170, 166, 4, 155, 3, 235, 26, 62, 8, 171, 19, 14, 53, 245, 77, 114, 175, 246, 170, 139,
            227, 212, 141, 72, 223, 134, 63, 91, 26, 12, 78, 253, 198, 162, 152, 202, 207, 170,
            254, 8, 4, 4, 175, 207, 84, 10, 108, 179, 157, 132, 110, 76, 201, 247, 227, 158, 106,
            59, 41, 206, 229, 128, 2, 60, 203, 65, 71, 160, 232, 186, 227, 51, 12, 142, 85, 93, 89,
            234, 236, 157, 230, 247, 167, 99, 7, 37, 146, 13, 53, 39, 255, 209, 177, 179, 17, 131,
            59, 16, 75, 180, 21, 119, 88, 4, 12, 49, 140, 3, 110, 235, 231, 92, 13, 41, 137, 21,
            37, 46, 138, 44, 250, 44, 161, 179, 114, 94, 63, 207, 192, 81, 234, 35, 125, 54, 2,
            214, 10, 57, 116, 154, 150, 147, 223, 232, 36, 108, 152, 145, 157, 132, 190, 103, 233,
            155, 141, 243, 249, 120, 72, 168, 14, 196, 35, 54, 107, 167, 218, 209, 1, 209, 197,
            187, 242, 76, 86, 229, 114, 131, 196, 69, 171, 118, 28, 51, 192, 146, 14, 140, 84, 66,
            155, 237, 194, 167, 121, 160, 166, 198, 166, 57, 13, 66, 162, 234, 148, 102, 133, 111,
            18, 166, 77, 156, 75, 84, 220, 80, 35, 81, 141, 23, 197, 162, 23, 167, 187, 187, 187,
            137, 184, 96, 140, 162, 6, 49, 63, 39, 84, 107, 85, 202, 168, 51, 194, 214, 132, 253,
            253, 189, 231, 1, 226, 118, 104, 84, 147, 244, 58, 233, 250, 66, 26, 109, 223, 34, 2,
            2, 112, 141, 147, 230, 134, 73, 45, 105, 180, 223, 52, 95, 40, 235, 209, 50, 67, 193,
            22, 176, 176, 128, 140, 238, 252, 129, 220, 175, 79, 133, 12, 123, 209, 64, 5, 160, 39,
            47, 66, 122, 245, 65, 102, 133, 58, 74, 138, 153, 217, 48, 59, 84, 135, 117, 92, 131,
            44, 109, 40, 105, 69, 29, 14, 142, 71, 87, 112, 68, 134, 0, 14, 158, 14, 68, 15, 180,
            150, 108, 49, 196, 94, 82, 27, 208, 163, 103, 81, 85, 124, 61, 242, 151, 29, 74, 87,
            134, 166, 145, 186, 110, 207, 162, 99, 92, 133, 121, 137, 124, 90, 134, 5, 249, 231,
            181, 222, 38, 170, 141, 113, 204, 172, 169, 173, 63, 81, 170, 76,
        ];
        let prunable_hash = Hash::from_slice(&[
            0x5c, 0x5e, 0x69, 0xd8, 0xfc, 0x0d, 0x22, 0x6a, 0x60, 0x91, 0x47, 0xda, 0x98, 0x36,
            0x06, 0x00, 0xf4, 0xea, 0x49, 0xcc, 0x49, 0x45, 0x2c, 0x5e, 0xf8, 0xba, 0x20, 0xf5,
            0x93, 0xd4, 0x80, 0x7d,
        ]);
        assert_eq!(prunable_hash, Hash::new(prunable_blob));
    }

    #[test]
    fn get_prunable_tx_blob() {
        let mut pruned_p_blob: Vec<u8> = vec![
            2, 0, 1, 2, 0, 16, 180, 149, 135, 30, 237, 231, 156, 1, 132, 145, 47, 182, 251, 153, 1,
            225, 234, 94, 219, 134, 23, 222, 210, 30, 208, 213, 12, 136, 158, 5, 159, 148, 15, 206,
            144, 2, 132, 63, 135, 22, 151, 8, 134, 8, 178, 26, 194, 111, 101, 192, 45, 104, 18,
            115, 178, 194, 100, 255, 227, 10, 253, 165, 53, 62, 81, 67, 202, 169, 56, 99, 42, 146,
            175, 137, 85, 195, 27, 151, 2, 0, 3, 207, 28, 183, 85, 7, 58, 81, 205, 53, 9, 191, 141,
            209, 70, 58, 30, 38, 225, 212, 68, 14, 4, 216, 204, 101, 163, 66, 156, 101, 143, 255,
            196, 134, 0, 3, 254, 66, 159, 187, 180, 41, 78, 252, 85, 255, 154, 55, 239, 222, 199,
            37, 159, 210, 71, 186, 188, 46, 134, 181, 236, 221, 173, 43, 93, 50, 138, 249, 221, 44,
            1, 34, 67, 111, 182, 199, 28, 219, 56, 238, 143, 188, 101, 103, 205, 139, 160, 144,
            226, 34, 92, 235, 221, 75, 38, 7, 104, 255, 108, 208, 1, 184, 169, 2, 9, 1, 84, 62, 77,
            107, 119, 22, 148, 222, 6, 128, 128, 211, 14, 242, 200, 16, 137, 239, 249, 55, 59, 16,
            193, 192, 140, 240, 153, 129, 228, 115, 222, 247, 41, 128, 219, 241, 249, 198, 214, 75,
            31, 82, 225, 1, 158, 183, 226, 220, 126, 228, 191, 211, 79, 43, 220, 95, 124, 109, 14,
            162, 170, 68, 37, 62, 21, 139, 182, 246, 152, 36, 156, 172, 197, 20, 145, 85, 9, 8,
            106, 237, 112, 63, 189, 172, 145, 49, 234, 68, 152, 200, 241, 0, 37,
        ];
        let prunable_blob: Vec<u8> = vec![
            1, 113, 10, 7, 87, 70, 119, 97, 244, 126, 155, 133, 254, 167, 60, 204, 134, 45, 71, 17,
            87, 21, 252, 8, 218, 233, 219, 192, 84, 181, 196, 74, 213, 2, 246, 222, 66, 45, 152,
            159, 156, 19, 224, 251, 110, 154, 188, 91, 129, 53, 251, 82, 134, 46, 93, 119, 136, 35,
            13, 190, 235, 231, 44, 183, 134, 221, 12, 131, 222, 209, 246, 52, 14, 33, 94, 173, 251,
            233, 18, 154, 91, 72, 229, 180, 43, 35, 152, 130, 38, 82, 56, 179, 36, 168, 54, 41, 62,
            49, 208, 35, 245, 29, 27, 81, 72, 140, 104, 4, 59, 22, 120, 252, 67, 197, 130, 245, 93,
            100, 129, 134, 19, 137, 228, 237, 166, 89, 5, 42, 1, 110, 139, 39, 81, 89, 159, 40,
            239, 211, 251, 108, 82, 68, 125, 182, 75, 152, 129, 74, 73, 208, 215, 15, 63, 3, 106,
            168, 35, 56, 126, 66, 2, 189, 53, 201, 77, 187, 102, 127, 154, 60, 209, 33, 217, 109,
            81, 217, 183, 252, 114, 90, 245, 21, 229, 174, 254, 177, 147, 130, 74, 49, 118, 203,
            14, 7, 118, 221, 81, 181, 78, 97, 224, 76, 160, 134, 73, 206, 204, 199, 201, 30, 201,
            77, 4, 78, 237, 167, 76, 92, 104, 247, 247, 203, 141, 243, 72, 52, 83, 61, 35, 147,
            231, 124, 21, 115, 81, 83, 67, 222, 61, 225, 171, 66, 243, 185, 195, 51, 72, 243, 80,
            104, 4, 166, 54, 199, 235, 193, 175, 4, 242, 42, 146, 170, 90, 212, 101, 208, 113, 58,
            65, 121, 55, 179, 206, 92, 50, 94, 171, 33, 67, 108, 220, 19, 193, 155, 30, 58, 46, 9,
            227, 48, 246, 187, 82, 230, 61, 64, 95, 197, 183, 150, 62, 203, 252, 36, 157, 135, 160,
            120, 189, 52, 94, 186, 93, 5, 36, 120, 160, 62, 254, 178, 101, 11, 228, 63, 128, 249,
            182, 56, 100, 9, 5, 2, 81, 243, 229, 245, 43, 234, 35, 216, 212, 46, 165, 251, 183,
            133, 10, 76, 172, 95, 106, 231, 13, 216, 222, 15, 92, 122, 103, 68, 238, 190, 108, 124,
            138, 62, 255, 243, 22, 209, 2, 138, 45, 178, 101, 240, 18, 186, 71, 239, 137, 191, 134,
            128, 221, 181, 173, 242, 111, 117, 45, 255, 138, 101, 79, 242, 42, 4, 144, 245, 193,
            79, 14, 44, 201, 223, 0, 193, 123, 75, 155, 140, 248, 0, 226, 246, 230, 126, 7, 32,
            107, 173, 193, 206, 184, 11, 33, 148, 104, 32, 79, 149, 71, 68, 150, 6, 47, 90, 231,
            151, 14, 121, 196, 169, 249, 117, 154, 167, 139, 103, 62, 97, 250, 131, 160, 92, 239,
            18, 236, 110, 184, 102, 30, 194, 175, 243, 145, 169, 183, 163, 141, 244, 186, 172, 251,
            3, 78, 165, 33, 12, 2, 136, 180, 178, 83, 117, 0, 184, 170, 255, 69, 131, 123, 8, 212,
            158, 162, 119, 137, 146, 63, 95, 133, 186, 91, 255, 152, 187, 107, 113, 147, 51, 219,
            207, 5, 160, 169, 97, 9, 1, 202, 152, 186, 128, 160, 110, 120, 7, 176, 103, 87, 30,
            137, 240, 67, 55, 79, 147, 223, 45, 177, 210, 101, 225, 22, 25, 129, 111, 101, 21, 213,
            20, 254, 36, 57, 67, 70, 93, 192, 11, 180, 75, 99, 185, 77, 75, 74, 63, 182, 183, 208,
            16, 69, 237, 96, 76, 96, 212, 242, 6, 169, 14, 250, 168, 129, 18, 141, 240, 101, 196,
            96, 120, 88, 90, 51, 77, 12, 133, 212, 192, 107, 131, 238, 34, 237, 93, 157, 108, 13,
            255, 187, 163, 106, 148, 108, 105, 244, 243, 174, 189, 180, 48, 102, 57, 170, 118, 211,
            110, 126, 222, 165, 93, 36, 157, 90, 14, 135, 184, 197, 185, 7, 99, 199, 224, 225, 243,
            212, 116, 149, 137, 186, 16, 196, 73, 23, 11, 248, 248, 67, 167, 149, 154, 64, 76, 218,
            119, 135, 239, 34, 48, 66, 57, 109, 246, 3, 141, 169, 42, 157, 222, 21, 40, 183, 168,
            97, 195, 106, 244, 229, 61, 122, 136, 59, 255, 120, 86, 30, 63, 226, 18, 65, 218, 188,
            195, 217, 85, 12, 211, 221, 188, 27, 8, 98, 103, 211, 213, 217, 65, 82, 229, 145, 80,
            147, 220, 57, 143, 20, 189, 253, 106, 13, 21, 170, 60, 24, 48, 162, 234, 0, 240, 226,
            4, 28, 76, 93, 56, 3, 187, 223, 58, 31, 184, 58, 234, 198, 140, 223, 217, 1, 147, 94,
            218, 199, 154, 121, 137, 44, 229, 0, 1, 10, 133, 250, 140, 64, 150, 89, 64, 112, 178,
            221, 87, 19, 24, 104, 252, 28, 65, 207, 28, 195, 217, 73, 12, 16, 83, 55, 199, 84, 117,
            175, 123, 13, 234, 10, 54, 63, 245, 161, 74, 235, 92, 189, 247, 47, 62, 176, 41, 159,
            40, 250, 116, 63, 33, 193, 78, 72, 29, 215, 9, 191, 233, 243, 87, 14, 195, 7, 89, 101,
            0, 28, 0, 234, 205, 59, 142, 119, 119, 52, 143, 80, 151, 211, 184, 235, 98, 222, 206,
            170, 166, 4, 155, 3, 235, 26, 62, 8, 171, 19, 14, 53, 245, 77, 114, 175, 246, 170, 139,
            227, 212, 141, 72, 223, 134, 63, 91, 26, 12, 78, 253, 198, 162, 152, 202, 207, 170,
            254, 8, 4, 4, 175, 207, 84, 10, 108, 179, 157, 132, 110, 76, 201, 247, 227, 158, 106,
            59, 41, 206, 229, 128, 2, 60, 203, 65, 71, 160, 232, 186, 227, 51, 12, 142, 85, 93, 89,
            234, 236, 157, 230, 247, 167, 99, 7, 37, 146, 13, 53, 39, 255, 209, 177, 179, 17, 131,
            59, 16, 75, 180, 21, 119, 88, 4, 12, 49, 140, 3, 110, 235, 231, 92, 13, 41, 137, 21,
            37, 46, 138, 44, 250, 44, 161, 179, 114, 94, 63, 207, 192, 81, 234, 35, 125, 54, 2,
            214, 10, 57, 116, 154, 150, 147, 223, 232, 36, 108, 152, 145, 157, 132, 190, 103, 233,
            155, 141, 243, 249, 120, 72, 168, 14, 196, 35, 54, 107, 167, 218, 209, 1, 209, 197,
            187, 242, 76, 86, 229, 114, 131, 196, 69, 171, 118, 28, 51, 192, 146, 14, 140, 84, 66,
            155, 237, 194, 167, 121, 160, 166, 198, 166, 57, 13, 66, 162, 234, 148, 102, 133, 111,
            18, 166, 77, 156, 75, 84, 220, 80, 35, 81, 141, 23, 197, 162, 23, 167, 187, 187, 187,
            137, 184, 96, 140, 162, 6, 49, 63, 39, 84, 107, 85, 202, 168, 51, 194, 214, 132, 253,
            253, 189, 231, 1, 226, 118, 104, 84, 147, 244, 58, 233, 250, 66, 26, 109, 223, 34, 2,
            2, 112, 141, 147, 230, 134, 73, 45, 105, 180, 223, 52, 95, 40, 235, 209, 50, 67, 193,
            22, 176, 176, 128, 140, 238, 252, 129, 220, 175, 79, 133, 12, 123, 209, 64, 5, 160, 39,
            47, 66, 122, 245, 65, 102, 133, 58, 74, 138, 153, 217, 48, 59, 84, 135, 117, 92, 131,
            44, 109, 40, 105, 69, 29, 14, 142, 71, 87, 112, 68, 134, 0, 14, 158, 14, 68, 15, 180,
            150, 108, 49, 196, 94, 82, 27, 208, 163, 103, 81, 85, 124, 61, 242, 151, 29, 74, 87,
            134, 166, 145, 186, 110, 207, 162, 99, 92, 133, 121, 137, 124, 90, 134, 5, 249, 231,
            181, 222, 38, 170, 141, 113, 204, 172, 169, 173, 63, 81, 170, 76,
        ];
        let mut tx_blob: Vec<u8> = Vec::new();
        tx_blob.append(&mut pruned_p_blob);
        tx_blob.append(&mut prunable_blob.clone());
        let mut buf = Vec::new();
        #[allow(clippy::expect_used)]
        let tx: monero::Transaction =
            monero::consensus::encode::deserialize(&tx_blob).expect("failed to serialize");
        #[allow(clippy::expect_used)]
        get_transaction_prunable_blob(&tx, &mut buf).expect("failed to get out prunable blob");
        assert_eq!(prunable_blob, buf);
    }
}
