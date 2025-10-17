use std::cmp::{max, min};

use indexmap::{IndexMap, IndexSet};
use monero_oxide::{
    io::CompressedPoint,
    transaction::{Input, Timelock},
};

use crate::{transactions::TransactionError, HardFork};

/// Gets the absolute offsets from the relative offsets.
///
/// This function will return an error if the relative offsets are empty.
/// <https://cuprate.github.io/monero-book/consensus_rules/transactions.html#inputs-must-have-decoys>
pub fn get_absolute_offsets(relative_offsets: &[u64]) -> Result<Vec<u64>, TransactionError> {
    if relative_offsets.is_empty() {
        return Err(TransactionError::InputDoesNotHaveExpectedNumbDecoys);
    }

    let mut offsets = Vec::with_capacity(relative_offsets.len());
    offsets.push(relative_offsets[0]);

    for i in 1..relative_offsets.len() {
        offsets.push(offsets[i - 1].wrapping_add(relative_offsets[i]));
    }
    Ok(offsets)
}

/// Inserts the output IDs that are needed to verify the transaction inputs into the provided `HashMap`.
///
/// This will error if the inputs are empty
/// <https://cuprate.github.io/monero-book/consensus_rules/transactions.html#no-empty-inputs>
///
pub fn insert_ring_member_ids(
    inputs: &[Input],
    output_ids: &mut IndexMap<u64, IndexSet<u64>>,
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
            Input::Gen(_) => return Err(TransactionError::IncorrectInputType),
        }
    }
    Ok(())
}

/// Represents the ring members of all the inputs.
#[derive(Debug)]
pub enum Rings {
    /// Legacy, pre-ringCT, rings.
    Legacy(Vec<Vec<CompressedPoint>>),
    /// `RingCT` rings, (outkey, amount commitment).
    RingCT(Vec<Vec<[CompressedPoint; 2]>>),
}

/// Information on the outputs the transaction is referencing for inputs (ring members).
#[derive(Debug)]
pub struct TxRingMembersInfo {
    pub rings: Rings,
    /// Information on the structure of the decoys, must be [`None`] for txs before [`HardFork::V1`]
    pub decoy_info: Option<DecoyInfo>,
    pub youngest_used_out_height: usize,
    pub time_locked_outs: Vec<Timelock>,
}

/// A struct holding information about the inputs and their decoys. This data can vary by block so
/// this data needs to be retrieved after every change in the blockchain.
///
/// This data *does not* need to be refreshed if one of these are true:
/// - The input amounts are *ALL* 0 (RCT)
/// - The top block hash is the same as when this data was retrieved (the blockchain state is unchanged).
///
/// <https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html>
#[derive(Debug, Copy, Clone)]
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
    /// `amount_outs_on_chain(inputs[X]) == outputs_with_amount[X]`
    ///
    /// Do not rely on this function to do consensus checks!
    ///
    pub fn new(
        inputs: &[Input],
        outputs_with_amount: impl Fn(u64) -> usize,
        hf: HardFork,
    ) -> Result<Self, TransactionError> {
        let mut min_decoys = usize::MAX;
        let mut max_decoys = usize::MIN;
        let mut mixable = 0;
        let mut not_mixable = 0;

        let minimum_decoys = minimum_decoys(hf);

        for inp in inputs {
            match inp {
                Input::ToKey {
                    key_offsets,
                    amount,
                    ..
                } => {
                    if let Some(amount) = amount {
                        let outs_with_amt = outputs_with_amount(*amount);

                        // <https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#mixable-and-unmixable-inputs>
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

                    // <https://cuprate.github.io/monero-book/consensus_rules/transactions/decoys.html#minimum-and-maximum-decoys-used>
                    min_decoys = min(min_decoys, numb_decoys);
                    max_decoys = max(max_decoys, numb_decoys);
                }
                Input::Gen(_) => return Err(TransactionError::IncorrectInputType),
            }
        }

        Ok(Self {
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
/// ref: <https://monero-book.cuprate.org/consensus_rules/transactions/inputs.html#default-minimum-decoys>
pub(crate) fn minimum_decoys(hf: HardFork) -> usize {
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
