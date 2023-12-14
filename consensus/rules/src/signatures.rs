use monero_serai::transaction::Transaction;

use crate::transactions::Rings;

mod ring_signatures;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    #[error("Number of signatures is different to the amount required.")]
    MismatchSignatureSize,
    #[error("The signature is incorrect.")]
    IncorrectSignature,
}

pub fn verify_contextual_signatures(tx: &Transaction, rings: &Rings) -> Result<(), SignatureError> {
    match rings {
        Rings::Legacy(_) => ring_signatures::verify_inputs_signatures(
            &tx.prefix.inputs,
            &tx.signatures,
            rings,
            &tx.signature_hash(),
        ),
        _ => panic!("TODO: RCT"),
    }
}
