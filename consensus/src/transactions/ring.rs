//! # Rings
//!
//! This module contains [`TxRingMembersInfo`] which is a struct made up from blockchain information about the
//! ring members of inputs. This module does minimal consensus checks, only when needed, and should not be relied
//! upon to do any.
//!

use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
    sync::Arc,
};

use curve25519_dalek::EdwardsPoint;
use monero_serai::{
    ringct::{mlsag::RingMatrix, RctType},
    transaction::{Input, Timelock, Transaction},
};
use tower::ServiceExt;

use crate::{
    transactions::TransactionVerificationData, ConsensusError, Database, DatabaseRequest,
    DatabaseResponse, HardFork, OutputOnChain,
};

/// Gets the absolute offsets from the relative offsets.
///
/// This function will return an error if the relative offsets are empty.
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#inputs-must-have-decoys
fn get_absolute_offsets(relative_offsets: &[u64]) -> Result<Vec<u64>, ConsensusError> {
    if relative_offsets.is_empty() {
        return Err(ConsensusError::TransactionHasInvalidRing(
            "ring has no members",
        ));
    }

    let mut offsets = Vec::with_capacity(relative_offsets.len());
    offsets.push(relative_offsets[0]);

    for i in 1..relative_offsets.len() {
        offsets.push(offsets[i - 1] + relative_offsets[i]);
    }
    Ok(offsets)
}

/// Inserts the outputs that are needed to verify the transaction inputs into the provided HashMap.
///
/// This will error if the inputs are empty
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#no-empty-inputs
///
pub fn insert_ring_member_ids(
    inputs: &[Input],
    output_ids: &mut HashMap<u64, HashSet<u64>>,
) -> Result<(), ConsensusError> {
    if inputs.is_empty() {
        return Err(ConsensusError::TransactionHasInvalidInput(
            "transaction has no inputs",
        ));
    }

    for input in inputs {
        match input {
            Input::ToKey {
                amount,
                key_offsets,
                ..
            } => output_ids
                .entry(amount.unwrap_or(0))
                .or_insert_with(HashSet::new)
                .extend(get_absolute_offsets(key_offsets)?),
            // https://cuprate.github.io/monero-book/consensus_rules/transactions.html#input-type
            _ => {
                return Err(ConsensusError::TransactionHasInvalidInput(
                    "input not ToKey",
                ))
            }
        }
    }
    Ok(())
}

/// Represents the ring members of all the inputs.
#[derive(Debug)]
pub enum Rings {
    /// Legacy, pre-ringCT, ring.
    Legacy(Vec<Vec<EdwardsPoint>>),
    /// TODO:
    RingCT,
}

impl Rings {
    /// Builds the rings for the transaction inputs, from the given outputs.
    pub fn new(outputs: Vec<Vec<&OutputOnChain>>, rct_type: RctType) -> Rings {
        match rct_type {
            RctType::Null => Rings::Legacy(
                outputs
                    .into_iter()
                    .map(|inp_outs| inp_outs.into_iter().map(|out| out.key).collect())
                    .collect(),
            ),
            _ => todo!("RingCT"),
        }
    }
}

/// Information on the outputs the transaction is is referencing for inputs (ring members).
#[derive(Debug)]
pub struct TxRingMembersInfo {
    pub rings: Rings,
    /// Information on the structure of the decoys, will be [`None`] for txs before [`HardFork::V1`]
    pub decoy_info: Option<DecoyInfo>,
    pub youngest_used_out_height: u64,
    pub time_locked_outs: Vec<Timelock>,
}

impl TxRingMembersInfo {
    pub fn new(
        used_outs: Vec<Vec<&OutputOnChain>>,
        decoy_info: Option<DecoyInfo>,
        rct_type: RctType,
    ) -> TxRingMembersInfo {
        TxRingMembersInfo {
            youngest_used_out_height: used_outs
                .iter()
                .map(|inp_outs| {
                    inp_outs
                        .iter()
                        .map(|out| out.height)
                        .max()
                        .expect("Input must have ring members")
                })
                .max()
                .expect("Tx must have inputs"),
            time_locked_outs: used_outs
                .iter()
                .flat_map(|inp_outs| {
                    inp_outs
                        .iter()
                        .filter_map(|out| match out.time_lock {
                            Timelock::None => None,
                            lock => Some(lock),
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            rings: Rings::new(used_outs, rct_type),
            decoy_info,
        }
    }
}

/// Get the ring members for the inputs from the outputs on the chain.
fn get_ring_members_for_inputs<'a>(
    outputs: &'a HashMap<u64, HashMap<u64, OutputOnChain>>,
    inputs: &[Input],
) -> Result<Vec<Vec<&'a OutputOnChain>>, ConsensusError> {
    inputs
        .iter()
        .map(|inp| match inp {
            Input::ToKey {
                amount,
                key_offsets,
                ..
            } => {
                let offsets = get_absolute_offsets(key_offsets)?;
                Ok(offsets
                    .iter()
                    .map(|offset| {
                        // get the hashmap for this amount.
                        outputs
                            .get(&amount.unwrap_or(0))
                            // get output at the index from the amount hashmap.
                            .and_then(|amount_map| amount_map.get(offset))
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
        .collect::<Result<_, ConsensusError>>()
}

/// Fills the `rings_member_info` field on the inputted [`TransactionVerificationData`]
pub async fn batch_fill_ring_member_info<D: Database + Clone + Send + Sync + 'static>(
    txs_verification_data: &[Arc<TransactionVerificationData>],
    hf: &HardFork,
    mut database: D,
) -> Result<(), ConsensusError> {
    let mut output_ids = HashMap::new();

    for tx_v_data in txs_verification_data.iter() {
        insert_ring_member_ids(&tx_v_data.tx.prefix.inputs, &mut output_ids)?;
    }

    let DatabaseResponse::Outputs(outputs) = database
        .ready()
        .await?
        .call(DatabaseRequest::Outputs(output_ids))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    for tx_v_data in txs_verification_data {
        let ring_members_for_tx =
            get_ring_members_for_inputs(&outputs, &tx_v_data.tx.prefix.inputs)?;

        let decoy_info = if hf != &HardFork::V1 {
            Some(DecoyInfo::new(&tx_v_data.tx.prefix.inputs, hf, database.clone()).await?)
        } else {
            None
        };

        // Temporarily acquirer the mutex lock to add the ring member info.
        let _ = tx_v_data
            .rings_member_info
            .lock()
            .unwrap()
            .insert(TxRingMembersInfo::new(
                ring_members_for_tx,
                decoy_info,
                tx_v_data.tx.rct_signatures.rct_type(),
            ));
    }

    Ok(())
}

/// A struct holding information about the inputs and their decoys.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html
#[derive(Debug)]
pub struct DecoyInfo {
    /// The number of inputs that have enough outputs on the chain to mix with.
    pub mixable: usize,
    /// The number of inputs that don't have enough outputs on the chain to mix with.
    pub not_mixable: usize,
    /// The minimum amount of decoys used in the transaction.
    pub min_decoys: usize,
    /// The maximum amount of decoys used in the transaction.
    pub max_decoys: usize,
}

impl DecoyInfo {
    /// Creates a new [`DecoyInfo`] struct relating to the passed in inputs.
    ///
    /// Do not rely on this function to do consensus checks!
    ///
    pub async fn new<D: Database>(
        inputs: &[Input],
        hf: &HardFork,
        mut database: D,
    ) -> Result<DecoyInfo, ConsensusError> {
        let mut min_decoys = usize::MAX;
        let mut max_decoys = usize::MIN;
        let mut mixable = 0;
        let mut not_mixable = 0;

        let minimum_decoys = minimum_decoys(hf);

        for inp in inputs {
            match inp {
                Input::ToKey {
                    amount,
                    key_offsets,
                    ..
                } => {
                    if let Some(amt) = *amount {
                        let DatabaseResponse::NumberOutputsWithAmount(numb_of_outs) = database
                            .ready()
                            .await?
                            .call(DatabaseRequest::NumberOutputsWithAmount(amt))
                            .await?
                        else {
                            panic!("Database sent incorrect response!");
                        };

                        // https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#mixable-and-unmixable-inputs
                        if numb_of_outs <= minimum_decoys && amt != 0 {
                            not_mixable += 1;
                        } else {
                            mixable += 1;
                        }
                    } else {
                        mixable += 1;
                    }

                    let numb_decoys = key_offsets
                        .len()
                        .checked_sub(1)
                        .ok_or(ConsensusError::TransactionHasInvalidRing("ring is empty"))?;
                    // https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#minimum-and-maximum-decoys-used
                    min_decoys = min(min_decoys, numb_decoys);
                    max_decoys = max(max_decoys, numb_decoys);
                }
                _ => {
                    return Err(ConsensusError::TransactionHasInvalidInput(
                        "input not ToKey",
                    ))
                }
            }
        }

        Ok(DecoyInfo {
            mixable,
            not_mixable,
            min_decoys,
            max_decoys,
        })
    }
}

/// Returns the minimum amount of decoys for a hard-fork.
/// **There are exceptions to this always being the minimum decoys**
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#minimum-amount-of-decoys
pub(crate) fn minimum_decoys(hf: &HardFork) -> usize {
    use HardFork::*;
    match hf {
        V1 => panic!("hard-fork 1 does not use these rules!"),
        V2 | V3 | V4 | V5 => 2,
        V6 => 4,
        V7 => 6,
        V8 | V9 | V10 | V11 | V12 | V13 | V14 => 10,
        _ => 15,
    }
}
