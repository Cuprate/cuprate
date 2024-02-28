//! Version 1 ring signature verification.
//!
//! Some checks have to be done at deserialization or with data we don't have so we can't do them here, those checks are:
//! <https://monero-book.cuprate.org/consensus_rules/transactions/ring_signatures.html#signatures-must-be-canonical>
//! this happens at deserialization in monero-serai.
//! <https://monero-book.cuprate.org/consensus_rules/transactions/ring_signatures.html#amount-of-signatures-in-a-ring>
//! and this happens during ring signature verification in monero-serai.
//!
use monero_serai::{ring_signatures::RingSignature, transaction::Input};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use super::{Rings, TransactionError};
use crate::try_par_iter;

/// Verifies the ring signature.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/ring_signatures.html>
pub fn check_input_signatures(
    inputs: &[Input],
    signatures: &[RingSignature],
    rings: &Rings,
    tx_sig_hash: &[u8; 32],
) -> Result<(), TransactionError> {
    match rings {
        Rings::Legacy(rings) => {
            // <https://monero-book.cuprate.org/consensus_rules/transactions/ring_signatures.html#amount-of-ring-signatures>
            // rings.len() != inputs.len() can't happen but check any way.
            if signatures.len() != inputs.len() || rings.len() != inputs.len() {
                return Err(TransactionError::RingSignatureIncorrect);
            }

            try_par_iter(inputs)
                .zip(rings)
                .zip(signatures)
                .try_for_each(|((input, ring), sig)| {
                    let Input::ToKey { key_image, .. } = input else {
                        panic!("How did we build a ring with no decoys?");
                    };

                    if !sig.verify(tx_sig_hash, ring, key_image) {
                        return Err(TransactionError::RingSignatureIncorrect);
                    }
                    Ok(())
                })?;
        }
        _ => panic!("tried to verify v1 tx with a non v1 ring"),
    }
    Ok(())
}
