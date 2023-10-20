use std::collections::{HashMap, HashSet};

use curve25519_dalek::EdwardsPoint;
use monero_serai::{
    ringct::{mlsag::RingMatrix, RctType},
    transaction::{Input, Transaction},
};
use tower::ServiceExt;

use crate::{hardforks::HardFork, ConsensusError, Database, DatabaseRequest, DatabaseResponse};

mod ring_sigs;

pub(crate) use ring_sigs::verify_inputs_signatures;

/// Gets the absolute offsets from the relative offsets.
/// This function will return an error if the relative offsets are empty or if the hf version is 6 or higher and
/// not all the ring members are unique.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#inputs-must-have-decoys
/// TODO: change the URL on this link \/
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#unique-inputs
fn get_absolute_offsets(
    relative_offsets: &[u64],
    hf: &HardFork,
) -> Result<Vec<u64>, ConsensusError> {
    if relative_offsets.is_empty() {
        return Err(ConsensusError::TransactionHasInvalidRing(
            "ring has no members",
        ));
    }

    let mut offsets = Vec::with_capacity(relative_offsets.len());
    offsets.push(relative_offsets[0]);

    for i in 1..relative_offsets.len() {
        if relative_offsets[i] == 0 && hf >= &HardFork::V6 {
            // all ring members must be unique after v6
            return Err(ConsensusError::TransactionHasInvalidRing(
                "ring has duplicate member",
            ));
        }

        offsets.push(relative_offsets[i - 1] + relative_offsets[i]);
    }
    Ok(offsets)
}

/// Returns the outputs that are needed to verify the transaction inputs.
///
/// The returned value is a hashmap with:
/// keys = amount
/// values = hashset of amount idxs
///
pub fn get_ring_member_ids(
    tx: &Transaction,
    hf: &HardFork,
) -> Result<HashMap<u64, HashSet<u64>>, ConsensusError> {
    let mut members = HashMap::with_capacity(tx.prefix.inputs.len());

    for input in &tx.prefix.inputs {
        match input {
            Input::ToKey {
                amount,
                key_offsets,
                ..
            } => members
                .entry(amount.unwrap_or(0))
                .or_insert_with(HashSet::new)
                .extend(get_absolute_offsets(key_offsets, hf)?),
            // https://cuprate.github.io/monero-book/consensus_rules/transactions.html#input-type
            _ => {
                return Err(ConsensusError::TransactionHasInvalidInput(
                    "input not ToKey",
                ))
            }
        }
    }

    // https://cuprate.github.io/monero-book/consensus_rules/transactions.html#no-empty-inputs
    if members.is_empty() {
        return Err(ConsensusError::TransactionHasInvalidInput(
            "transaction has no inputs",
        ));
    }

    Ok(members)
}

/// Represents the ring members of the inputs.
pub enum Rings {
    /// Legacy, pre-ringCT, ring.
    Legacy(Vec<Vec<EdwardsPoint>>),
    /// TODO:
    RingCT,
}

impl Rings {
    /// Builds the rings for the transaction inputs, from the outputs.
    pub fn new(
        outputs: &HashMap<u64, HashMap<u64, [EdwardsPoint; 2]>>,
        inputs: &[Input],
        rct_type: RctType,
        hf: &HardFork,
    ) -> Result<Rings, ConsensusError> {
        match rct_type {
            RctType::Null => {
                let legacy_ring = inputs
                    .iter()
                    .map(|inp| match inp {
                        Input::ToKey {
                            amount,
                            key_offsets,
                            ..
                        } => {
                            let offsets = get_absolute_offsets(key_offsets, hf)?;
                            Ok(offsets
                                .iter()
                                .map(|offset| {
                                    // get the hashmap for this amount.
                                    outputs
                                        .get(&amount.unwrap_or(0))
                                        // get output at the index from the amount hashmap.
                                        .and_then(|amount_map| amount_map.get(offset))
                                        // this is a legacy ring we only need the one time key.
                                        .and_then(|out| Some(out[0]))
                                        .ok_or(ConsensusError::TransactionHasInvalidRing(
                                            "ring member not in database",
                                        ))
                                })
                                .collect::<Result<_, ConsensusError>>()?)
                        }
                        _ => Err(ConsensusError::TransactionHasInvalidInput(
                            "input not ToKey",
                        )),
                    })
                    .collect::<Result<_, ConsensusError>>()?;

                Ok(Rings::Legacy(legacy_ring))
            }
            _ => todo!("RingCT"),
        }
    }
}

/// Get [`Rings`] aka the outputs a transaction references for each transaction.
pub async fn batch_get_rings<D: Database>(
    txs: &[Transaction],
    hf: &HardFork,
    database: D,
) -> Result<Vec<Rings>, ConsensusError> {
    let mut output_ids = HashMap::new();

    for tx in txs {
        let mut tx_out_ids = get_ring_member_ids(tx, hf)?;
        for (amount, idxs) in tx_out_ids.drain() {
            output_ids
                .entry(amount)
                .or_insert_with(HashSet::new)
                .extend(idxs);
        }
    }

    let DatabaseResponse::Outputs(outputs) = database
        .oneshot(DatabaseRequest::Outputs(output_ids))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    let mut rings = Vec::with_capacity(txs.len());

    for tx in txs {
        rings.push(Rings::new(
            &outputs,
            &tx.prefix.inputs,
            tx.rct_signatures.rct_type(),
            hf,
        )?);
    }

    Ok(rings)
}
