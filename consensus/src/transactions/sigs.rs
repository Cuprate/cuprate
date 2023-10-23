use std::sync::Arc;

use monero_serai::transaction::Transaction;
use multiexp::BatchVerifier as CoreBatchVerifier;

use crate::{transactions::ring::Rings, ConsensusError};

mod ring_sigs;

#[derive(Clone)]
pub struct BatchVerifier {
    batch_verifier: Arc<std::sync::Mutex<CoreBatchVerifier<u64, dalek_ff_group::EdwardsPoint>>>,
}

pub struct BatchVerifierHandle {
    batch_verifier: BatchVerifier,
}

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
