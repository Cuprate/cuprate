use curve25519_dalek::EdwardsPoint;
use monero_serai::transaction::Transaction;

mod ring_signatures;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    #[error("Number of signatures is different to the amount required.")]
    MismatchSignatureSize,
    #[error("The signature is incorrect.")]
    IncorrectSignature,
}

/// Represents the ring members of all the inputs.
#[derive(Debug)]
pub enum Rings {
    /// Legacy, pre-ringCT, rings.
    Legacy(Vec<Vec<EdwardsPoint>>),
    // RingCT rings, (outkey, amount commitment).
    RingCT(Vec<Vec<[EdwardsPoint; 2]>>),
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
