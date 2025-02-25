use curve25519_dalek::{EdwardsPoint, Scalar};
use hex_literal::hex;
use monero_serai::io::decompress_point;
use monero_serai::{
    generators::H,
    ringct::{
        clsag::ClsagError,
        mlsag::{AggregateRingMatrixBuilder, MlsagError, RingMatrix},
        RctProofs, RctPrunable, RctType,
    },
    transaction::Input,
};
use rand::thread_rng;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::{batch_verifier::BatchVerifier, transactions::Rings, try_par_iter, HardFork};

/// This constant contains the IDs of 2 transactions that should be allowed after the fork the ringCT
/// type they used should be banned.
const GRANDFATHERED_TRANSACTIONS: [[u8; 32]; 2] = [
    hex!("c5151944f0583097ba0c88cd0f43e7fabb3881278aa2f73b3b0a007c5d34e910"),
    hex!("6f2f117cde6fbcf8d4a6ef8974fcac744726574ac38cf25d3322c996b21edd4c"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RingCTError {
    #[error("The RingCT type used is not allowed.")]
    TypeNotAllowed,
    #[error("RingCT simple: sum pseudo-outs does not equal outputs.")]
    SimpleAmountDoNotBalance,
    #[error("The borromean range proof is invalid.")]
    BorromeanRangeInvalid,
    #[error("The bulletproofs range proof is invalid.")]
    BulletproofsRangeInvalid,
    #[error("One or more input ring is invalid.")]
    RingInvalid,
    #[error("MLSAG Error: {0}.")]
    MLSAGError(#[from] MlsagError),
    #[error("CLSAG Error: {0}.")]
    CLSAGError(#[from] ClsagError),
}

/// Checks the `RingCT` type is allowed for the current hard fork.
///
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct.html#type>
fn check_rct_type(ty: RctType, hf: HardFork, tx_hash: &[u8; 32]) -> Result<(), RingCTError> {
    use HardFork as F;
    use RctType as T;

    match ty {
        T::AggregateMlsagBorromean | T::MlsagBorromean if hf >= F::V4 && hf < F::V9 => Ok(()),
        T::MlsagBulletproofs if hf >= F::V8 && hf < F::V11 => Ok(()),
        T::MlsagBulletproofsCompactAmount if hf >= F::V10 && hf < F::V14 => Ok(()),
        T::MlsagBulletproofsCompactAmount if GRANDFATHERED_TRANSACTIONS.contains(tx_hash) => Ok(()),
        T::ClsagBulletproof if hf >= F::V13 && hf < F::V16 => Ok(()),
        T::ClsagBulletproofPlus if hf >= F::V15 => Ok(()),
        _ => Err(RingCTError::TypeNotAllowed),
    }
}

/// Checks that the pseudo-outs sum to the same point as the output commitments.
///
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct.html#pseudo-outs-outpks-balance>
fn simple_type_balances(rct_sig: &RctProofs) -> Result<(), RingCTError> {
    let pseudo_outs = if rct_sig.rct_type() == RctType::MlsagBorromean {
        &rct_sig.base.pseudo_outs
    } else {
        match &rct_sig.prunable {
            RctPrunable::Clsag { pseudo_outs, .. }
            | RctPrunable::MlsagBulletproofsCompactAmount { pseudo_outs, .. }
            | RctPrunable::MlsagBulletproofs { pseudo_outs, .. } => pseudo_outs,
            RctPrunable::MlsagBorromean { .. } => &rct_sig.base.pseudo_outs,
            RctPrunable::AggregateMlsagBorromean { .. } => panic!("RingCT type is not simple!"),
        }
    };

    let sum_inputs = pseudo_outs
        .iter()
        .copied()
        .map(decompress_point)
        .sum::<Option<EdwardsPoint>>()
        .ok_or(RingCTError::SimpleAmountDoNotBalance)?;
    let sum_outputs = rct_sig
        .base
        .commitments
        .iter()
        .copied()
        .map(decompress_point)
        .sum::<Option<EdwardsPoint>>()
        .ok_or(RingCTError::SimpleAmountDoNotBalance)?
        + Scalar::from(rct_sig.base.fee) * *H;

    if sum_inputs == sum_outputs {
        Ok(())
    } else {
        Err(RingCTError::SimpleAmountDoNotBalance)
    }
}

/// Checks the outputs range proof(s)
///
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct/borromean.html>
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct/bulletproofs.html>
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct/bulletproofs+.html>
fn check_output_range_proofs(
    proofs: &RctProofs,
    mut verifier: impl BatchVerifier,
) -> Result<(), RingCTError> {
    let commitments = &proofs.base.commitments;

    match &proofs.prunable {
        RctPrunable::MlsagBorromean { borromean, .. }
        | RctPrunable::AggregateMlsagBorromean { borromean, .. } => try_par_iter(borromean)
            .zip(commitments)
            .try_for_each(|(borro, commitment)| {
                if borro.verify(commitment) {
                    Ok(())
                } else {
                    Err(RingCTError::BorromeanRangeInvalid)
                }
            }),
        RctPrunable::MlsagBulletproofs { bulletproof, .. }
        | RctPrunable::MlsagBulletproofsCompactAmount { bulletproof, .. }
        | RctPrunable::Clsag { bulletproof, .. } => {
            if verifier.queue_statement(|verifier| {
                bulletproof.batch_verify(&mut thread_rng(), verifier, commitments)
            }) {
                Ok(())
            } else {
                Err(RingCTError::BulletproofsRangeInvalid)
            }
        }
    }
}

pub(crate) fn ring_ct_semantic_checks(
    proofs: &RctProofs,
    tx_hash: &[u8; 32],
    verifier: impl BatchVerifier,
    hf: HardFork,
) -> Result<(), RingCTError> {
    let rct_type = proofs.rct_type();

    check_rct_type(rct_type, hf, tx_hash)?;
    check_output_range_proofs(proofs, verifier)?;

    if rct_type != RctType::AggregateMlsagBorromean {
        simple_type_balances(proofs)?;
    }

    Ok(())
}

/// Check the input signatures: MLSAG, CLSAG.
///
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct/mlsag.html>
/// <https://monero-book.cuprate.org/consensus_rules/ring_ct/clsag.html>
pub(crate) fn check_input_signatures(
    msg: &[u8; 32],
    inputs: &[Input],
    proofs: &RctProofs,
    rings: &Rings,
) -> Result<(), RingCTError> {
    let Rings::RingCT(rings) = rings else {
        panic!("Tried to verify RCT transaction without RCT ring");
    };

    if rings.is_empty() {
        return Err(RingCTError::RingInvalid);
    }

    let pseudo_outs = match &proofs.prunable {
        RctPrunable::MlsagBulletproofs { pseudo_outs, .. }
        | RctPrunable::MlsagBulletproofsCompactAmount { pseudo_outs, .. }
        | RctPrunable::Clsag { pseudo_outs, .. } => pseudo_outs.as_slice(),
        RctPrunable::MlsagBorromean { .. } => proofs.base.pseudo_outs.as_slice(),
        RctPrunable::AggregateMlsagBorromean { .. } => &[],
    };

    match &proofs.prunable {
        RctPrunable::AggregateMlsagBorromean { mlsag, .. } => {
            let key_images = inputs
                .iter()
                .map(|inp| {
                    let Input::ToKey { key_image, .. } = inp else {
                        panic!("How did we build a ring with no decoys?");
                    };
                    *key_image
                })
                .collect::<Vec<_>>();

            let mut matrix =
                AggregateRingMatrixBuilder::new(&proofs.base.commitments, proofs.base.fee)?;

            rings.iter().try_for_each(|ring| matrix.push_ring(ring))?;

            Ok(mlsag.verify(msg, &matrix.build()?, &key_images)?)
        }
        RctPrunable::MlsagBorromean { mlsags, .. }
        | RctPrunable::MlsagBulletproofsCompactAmount { mlsags, .. }
        | RctPrunable::MlsagBulletproofs { mlsags, .. } => try_par_iter(mlsags)
            .zip(pseudo_outs)
            .zip(inputs)
            .zip(rings)
            .try_for_each(|(((mlsag, pseudo_out), input), ring)| {
                let Input::ToKey { key_image, .. } = input else {
                    panic!("How did we build a ring with no decoys?");
                };

                Ok(mlsag.verify(
                    msg,
                    &RingMatrix::individual(ring, *pseudo_out)?,
                    &[*key_image],
                )?)
            }),
        RctPrunable::Clsag { clsags, .. } => try_par_iter(clsags)
            .zip(pseudo_outs)
            .zip(inputs)
            .zip(rings)
            .try_for_each(|(((clsags, pseudo_out), input), ring)| {
                let Input::ToKey { key_image, .. } = input else {
                    panic!("How did we build a ring with no decoys?");
                };

                Ok(clsags.verify(ring.clone(), key_image, pseudo_out, msg)?)
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grandfathered_bulletproofs2() {
        assert!(check_rct_type(
            RctType::MlsagBulletproofsCompactAmount,
            HardFork::V14,
            &[0; 32]
        )
        .is_err());

        assert!(check_rct_type(
            RctType::MlsagBulletproofsCompactAmount,
            HardFork::V14,
            &GRANDFATHERED_TRANSACTIONS[0]
        )
        .is_ok());
        assert!(check_rct_type(
            RctType::MlsagBulletproofsCompactAmount,
            HardFork::V14,
            &GRANDFATHERED_TRANSACTIONS[1]
        )
        .is_ok());
    }
}
