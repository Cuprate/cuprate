use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
};

use curve25519_dalek::EdwardsPoint;
use monero_serai::transaction::{Input, Timelock};

use crate::{transactions::TransactionError, HardFork, TxVersion};

/// An already approved previous transaction output.
#[derive(Debug)]
pub struct OutputOnChain {
    pub height: u64,
    pub time_lock: Timelock,
    pub key: Option<EdwardsPoint>,
    pub commitment: EdwardsPoint,
}

/// Gets the absolute offsets from the relative offsets.
///
/// This function will return an error if the relative offsets are empty.
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#inputs-must-have-decoys
fn get_absolute_offsets(relative_offsets: &[u64]) -> Result<Vec<u64>, TransactionError> {
    if relative_offsets.is_empty() {
        return Err(TransactionError::InputDoesNotHaveExpectedNumbDecoys);
    }

    let mut offsets = Vec::with_capacity(relative_offsets.len());
    offsets.push(relative_offsets[0]);

    for i in 1..relative_offsets.len() {
        offsets.push(offsets[i - 1] + relative_offsets[i]);
    }
    Ok(offsets)
}

/// Inserts the output IDs that are needed to verify the transaction inputs into the provided HashMap.
///
/// This will error if the inputs are empty
/// https://cuprate.github.io/monero-book/consensus_rules/transactions.html#no-empty-inputs
///
pub fn insert_ring_member_ids(
    inputs: &[Input],
    output_ids: &mut HashMap<u64, HashSet<u64>>,
) -> Result<(), TransactionError> {
    if inputs.is_empty() {
        return Err(TransactionError::NoInputs);
    }

    for input in inputs {
        match input {
            Input::ToKey {
                amount,
                key_offsets,
                ..
            } => output_ids
                .entry(amount.unwrap_or(0))
                .or_default()
                .extend(get_absolute_offsets(key_offsets)?),
            _ => return Err(TransactionError::IncorrectInputType),
        }
    }
    Ok(())
}

/// Get the ring members for the inputs from the outputs on the chain.
///
/// Will error if `outputs` does not contain the outputs needed.
pub fn get_ring_members_for_inputs<'a>(
    get_outputs: impl Fn(u64, u64) -> Option<&'a OutputOnChain>,
    inputs: &[Input],
) -> Result<Vec<Vec<&'a OutputOnChain>>, TransactionError> {
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
                        get_outputs(amount.unwrap_or(0), *offset)
                            .ok_or(TransactionError::RingMemberNotFoundOrInvalid)
                    })
                    .collect::<Result<_, TransactionError>>()?)
            }
            _ => Err(TransactionError::IncorrectInputType),
        })
        .collect::<Result<_, TransactionError>>()
}

/// Represents the ring members of all the inputs.
#[derive(Debug)]
pub enum Rings {
    /// Legacy, pre-ringCT, rings.
    Legacy(Vec<Vec<EdwardsPoint>>),
    /// RingCT rings, (outkey, amount commitment).
    RingCT(Vec<Vec<[EdwardsPoint; 2]>>),
}

impl Rings {
    /// Builds the rings for the transaction inputs, from the given outputs.
    fn new(
        outputs: Vec<Vec<&OutputOnChain>>,
        tx_version: TxVersion,
    ) -> Result<Rings, TransactionError> {
        Ok(match tx_version {
            TxVersion::RingSignatures => Rings::Legacy(
                outputs
                    .into_iter()
                    .map(|inp_outs| {
                        inp_outs
                            .into_iter()
                            .map(|out| out.key.ok_or(TransactionError::RingMemberNotFoundOrInvalid))
                            .collect::<Result<Vec<_>, TransactionError>>()
                    })
                    .collect::<Result<Vec<_>, TransactionError>>()?,
            ),
            TxVersion::RingCT => Rings::RingCT(
                outputs
                    .into_iter()
                    .map(|inp_outs| {
                        inp_outs
                            .into_iter()
                            .map(|out| {
                                Ok([
                                    out.key
                                        .ok_or(TransactionError::RingMemberNotFoundOrInvalid)?,
                                    out.commitment,
                                ])
                            })
                            .collect::<Result<_, TransactionError>>()
                    })
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
}

/// Information on the outputs the transaction is is referencing for inputs (ring members).
#[derive(Debug)]
pub struct TxRingMembersInfo {
    pub rings: Rings,
    /// Information on the structure of the decoys, must be [`None`] for txs before [`HardFork::V1`]
    pub decoy_info: Option<DecoyInfo>,
    pub youngest_used_out_height: u64,
    pub time_locked_outs: Vec<Timelock>,
    pub hf: HardFork,
}

impl TxRingMembersInfo {
    /// Construct a [`TxRingMembersInfo`] struct.
    ///
    /// The used outs must be all the ring members used in the transactions inputs.
    pub fn new(
        used_outs: Vec<Vec<&OutputOnChain>>,
        decoy_info: Option<DecoyInfo>,
        tx_version: TxVersion,
        hf: HardFork,
    ) -> Result<TxRingMembersInfo, TransactionError> {
        Ok(TxRingMembersInfo {
            youngest_used_out_height: used_outs
                .iter()
                .map(|inp_outs| {
                    inp_outs
                        .iter()
                        // the output with the highest height is the youngest
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
            hf,
            rings: Rings::new(used_outs, tx_version)?,
            decoy_info,
        })
    }
}

/// A struct holding information about the inputs and their decoys. This data can vary by block so
/// this data needs to be retrieved after every change in the blockchain.
///
/// This data *does not* need to be refreshed if one of these are true:
/// - The input amounts are *ALL* 0 (RCT)
/// - The top block hash is the same as when this data was retrieved (the blockchain state is unchanged).
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
    /// Creates a new [`DecoyInfo`] struct relating to the passed in inputs, This is only needed from
    /// hf 2 onwards.
    ///
    /// `outputs_with_amount` is a list of the amount of outputs currently on the chain with the same amount
    /// as the `inputs` amount at the same index. For RCT inputs it instead should be [`None`].
    ///
    /// So:
    ///
    /// amount_outs_on_chain(inputs`[X]`) == outputs_with_amount`[X]`
    ///
    /// Do not rely on this function to do consensus checks!
    ///
    pub fn new(
        inputs: &[Input],
        outputs_with_amount: &HashMap<u64, usize>,
        hf: &HardFork,
    ) -> Result<DecoyInfo, TransactionError> {
        let mut min_decoys = usize::MAX;
        let mut max_decoys = usize::MIN;
        let mut mixable = 0;
        let mut not_mixable = 0;

        let minimum_decoys = minimum_decoys(hf);

        for inp in inputs.iter() {
            match inp {
                Input::ToKey {
                    key_offsets,
                    amount,
                    ..
                } => {
                    if let Some(amount) = amount {
                        let outs_with_amt = *outputs_with_amount
                            .get(amount)
                            .expect("outputs_with_amount does not include needed amount.");

                        // https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#mixable-and-unmixable-inputs
                        if outs_with_amt <= minimum_decoys {
                            not_mixable += 1;
                        } else {
                            mixable += 1;
                        }
                    } else {
                        // ringCT amounts are always mixable.
                        mixable += 1;
                    }

                    let numb_decoys = key_offsets
                        .len()
                        .checked_sub(1)
                        .ok_or(TransactionError::InputDoesNotHaveExpectedNumbDecoys)?;

                    // https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#minimum-and-maximum-decoys-used
                    min_decoys = min(min_decoys, numb_decoys);
                    max_decoys = max(max_decoys, numb_decoys);
                }
                _ => return Err(TransactionError::IncorrectInputType),
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

/// Returns the default minimum amount of decoys for a hard-fork.
/// **There are exceptions to this always being the minimum decoys**
///
/// ref: https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#default-minimum-decoys
pub(crate) fn minimum_decoys(hf: &HardFork) -> usize {
    use HardFork as HF;
    match hf {
        HF::V1 => panic!("hard-fork 1 does not use these rules!"),
        HF::V2 | HF::V3 | HF::V4 | HF::V5 => 2,
        HF::V6 => 4,
        HF::V7 => 6,
        HF::V8 | HF::V9 | HF::V10 | HF::V11 | HF::V12 | HF::V13 | HF::V14 => 10,
        HF::V15 | HF::V16 => 15,
    }
}
