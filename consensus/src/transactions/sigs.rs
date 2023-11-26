use monero_serai::transaction::Transaction;

use crate::{transactions::contextual_data::Rings, ConsensusError};

mod ring_sigs;

pub fn verify_signatures(tx: &Transaction, rings: &Rings) -> Result<(), ConsensusError> {
    match rings {
        Rings::Legacy(_) => ring_sigs::verify_inputs_signatures(
            &tx.prefix.inputs,
            &tx.signatures,
            rings,
            &tx.signature_hash(),
        ),
        _ => panic!("TODO: RCT"),
    }
}
