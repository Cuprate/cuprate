//! Version 1 ring signature verification.
//!
//! Some checks have to be done at deserialization or with data we don't have so we can't do them here, those checks are:
//! https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#signatures-must-be-canonical
//! this happens at deserialization in monero-serai.
//! https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#amount-of-signatures-in-a-ring
//! and this happens during ring signature verification in monero-serai.
//!
use monero_serai::{ring_signatures::RingSignature, transaction::Input};
use rayon::prelude::*;

use super::Rings;
use crate::ConsensusError;

/// Verifies the ring signature.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#the-ring-signature-must-be-valid
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/pre_rct.html#amount-of-ring-signatures
pub fn verify_inputs_signatures(
    inputs: &[Input],
    signatures: &[RingSignature],
    rings: &Rings,
    tx_sig_hash: &[u8; 32],
) -> Result<(), ConsensusError> {
    match rings {
        Rings::Legacy(rings) => {
            // rings.len() != inputs.len() can't happen but check any way.
            if signatures.len() != inputs.len() || rings.len() != inputs.len() {
                return Err(ConsensusError::TransactionSignatureInvalid(
                    "number of ring sigs != inputs",
                ));
            }

            inputs
                .par_iter()
                .zip(rings)
                .zip(signatures)
                .try_for_each(|((input, ring), sig)| {
                    let Input::ToKey { key_image, .. } = input else {
                        panic!("How did we build a ring with no decoys?");
                    };

                    if !sig.verify(tx_sig_hash, ring, key_image) {
                        return Err(ConsensusError::TransactionSignatureInvalid(
                            "Invalid ring signature",
                        ));
                    }
                    Ok(())
                })?;
        }
        _ => panic!("tried to verify v1 tx with a non v1 ring"),
    }
    Ok(())
}
