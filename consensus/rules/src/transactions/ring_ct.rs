use curve25519_dalek::{EdwardsPoint, Scalar};
use hex_literal::hex;
use monero_serai::{
    ringct::{
        clsag::ClsagError,
        mlsag::{AggregateRingMatrixBuilder, MlsagError, RingMatrix},
        RctPrunable, RctSignatures, RctType,
    },
    transaction::{Input, Output, Transaction},
    H,
};
use multiexp::BatchVerifier;
use rand::thread_rng;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::{transactions::Rings, try_par_iter, HardFork};

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
    #[error("One or more of the outputs do not have a zero amount.")]
    OutputNotZero,
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

/// Checks the RingCT type is allowed for the current hard fork.
///
/// https://monero-book.cuprate.org/consensus_rules/ring_ct.html#type
fn check_rct_type(ty: &RctType, hf: HardFork, tx_hash: &[u8; 32]) -> Result<(), RingCTError> {
    use HardFork as F;
    use RctType as T;

    match ty {
        T::MlsagAggregate | T::MlsagIndividual if hf >= F::V4 && hf < F::V9 => Ok(()),
        T::Bulletproofs if hf >= F::V8 && hf < F::V11 => Ok(()),
        T::BulletproofsCompactAmount if hf >= F::V10 && hf < F::V14 => Ok(()),
        T::BulletproofsCompactAmount if GRANDFATHERED_TRANSACTIONS.contains(tx_hash) => Ok(()),
        T::Clsag if hf >= F::V13 && hf < F::V16 => Ok(()),
        T::BulletproofsPlus if hf >= F::V15 => Ok(()),
        _ => Err(RingCTError::TypeNotAllowed),
    }
}

/// Checks all the outputs have a zero amount.
///
/// https://monero-book.cuprate.org/consensus_rules/ring_ct.html#output-amount
fn check_output_amount(outputs: &[Output]) -> Result<(), RingCTError> {
    outputs.iter().try_for_each(|out| {
        if out.amount.is_none() {
            Ok(())
        } else {
            Err(RingCTError::OutputNotZero)
        }
    })
}

/// Checks that the pseudo-outs sum to the same point as the output commitments.
///
/// https://monero-book.cuprate.org/consensus_rules/ring_ct.html#pseudo-outs-outpks-balance
fn simple_type_balances(rct_sig: &RctSignatures) -> Result<(), RingCTError> {
    let pseudo_outs = if rct_sig.rct_type() == RctType::MlsagIndividual {
        &rct_sig.base.pseudo_outs
    } else {
        match &rct_sig.prunable {
            RctPrunable::Clsag { pseudo_outs, .. }
            | RctPrunable::MlsagBulletproofs { pseudo_outs, .. } => pseudo_outs,
            _ => panic!("RingCT type is not simple!"),
        }
    };

    let sum_inputs = pseudo_outs.iter().sum::<EdwardsPoint>();
    let sum_outputs = rct_sig.base.commitments.iter().sum::<EdwardsPoint>()
        + Scalar::from(rct_sig.base.fee) * H();

    if sum_inputs == sum_outputs {
        Ok(())
    } else {
        Err(RingCTError::SimpleAmountDoNotBalance)
    }
}

/// Checks the outputs range proof(s)
///
/// https://monero-book.cuprate.org/consensus_rules/ring_ct/borromean.html
/// https://monero-book.cuprate.org/consensus_rules/ring_ct/bulletproofs.html
/// https://monero-book.cuprate.org/consensus_rules/ring_ct/bulletproofs+.html
fn check_output_range_proofs(
    rct_sig: &RctSignatures,
    verifier: &mut BatchVerifier<(), dalek_ff_group::EdwardsPoint>,
) -> Result<(), RingCTError> {
    let commitments = &rct_sig.base.commitments;

    match &rct_sig.prunable {
        RctPrunable::Null => Err(RingCTError::TypeNotAllowed)?,
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
        RctPrunable::MlsagBulletproofs { bulletproofs, .. }
        | RctPrunable::Clsag { bulletproofs, .. } => {
            if bulletproofs.batch_verify(&mut thread_rng(), verifier, (), commitments) {
                Ok(())
            } else {
                Err(RingCTError::BulletproofsRangeInvalid)
            }
        }
    }
}

pub fn ring_ct_semantic_checks(
    tx: &Transaction,
    tx_hash: &[u8; 32],
    verifier: &mut BatchVerifier<(), dalek_ff_group::EdwardsPoint>,
    hf: &HardFork,
) -> Result<(), RingCTError> {
    check_output_amount(&tx.prefix.outputs)?;
    check_rct_type(&tx.rct_signatures.rct_type(), *hf, tx_hash)?;
    check_output_range_proofs(&tx.rct_signatures, verifier)?;

    if tx.rct_signatures.rct_type() != RctType::MlsagAggregate {
        simple_type_balances(&tx.rct_signatures)?;
    }

    Ok(())
}

/// Check the input signatures, MLSAG, CLSAG.
///
/// https://monero-book.cuprate.org/consensus_rules/ring_ct/mlsag.html
/// https://monero-book.cuprate.org/consensus_rules/ring_ct/clsag.html
pub fn check_input_signatures(
    msg: &[u8; 32],
    inputs: &[Input],
    rct_sig: &RctSignatures,
    rings: &Rings,
) -> Result<(), RingCTError> {
    let Rings::RingCT(rings) = rings else {
        panic!("Tried to verify RCT transaction without RCT ring");
    };

    if rings.is_empty() {
        Err(RingCTError::RingInvalid)?;
    }

    match &rct_sig.prunable {
        RctPrunable::Null => Err(RingCTError::TypeNotAllowed)?,
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
                AggregateRingMatrixBuilder::new(&rct_sig.base.commitments, rct_sig.base.fee);
            rings.iter().try_for_each(|ring| matrix.push_ring(ring))?;
            Ok(mlsag.verify(msg, &matrix.build()?, &key_images)?)
        }
        RctPrunable::MlsagBorromean { mlsags, .. }
        | RctPrunable::MlsagBulletproofs { mlsags, .. } => try_par_iter(mlsags)
            .zip(&rct_sig.base.pseudo_outs)
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
            .zip(&rct_sig.base.pseudo_outs)
            .zip(inputs)
            .zip(rings)
            .try_for_each(|(((clsags, pseudo_out), input), ring)| {
                let Input::ToKey { key_image, .. } = input else {
                    panic!("How did we build a ring with no decoys?");
                };

                Ok(clsags.verify(ring, key_image, pseudo_out, msg)?)
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grandfathered_bulletproofs2() {
        assert!(
            check_rct_type(&RctType::BulletproofsCompactAmount, HardFork::V14, &[0; 32]).is_err()
        );

        assert!(check_rct_type(
            &RctType::BulletproofsCompactAmount,
            HardFork::V14,
            &GRANDFATHERED_TRANSACTIONS[0]
        )
        .is_ok());
        assert!(check_rct_type(
            &RctType::BulletproofsCompactAmount,
            HardFork::V14,
            &GRANDFATHERED_TRANSACTIONS[1]
        )
        .is_ok());
    }
}
