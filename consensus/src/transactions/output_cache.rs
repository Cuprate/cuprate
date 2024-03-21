use std::{
    collections::{BTreeMap, HashMap},
    iter::once,
    sync::Arc,
};

use curve25519_dalek::{
    constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY, EdwardsPoint, Scalar,
};
use monero_serai::{
    block::Block,
    transaction::{Input, Timelock},
    H,
};
use tower::ServiceExt;

use cuprate_consensus_rules::{
    blocks::BlockError,
    miner_tx::MinerTxError,
    transactions::{OutputOnChain, TransactionError},
    ConsensusError,
};

use crate::{
    transactions::TransactionVerificationData, Database, DatabaseRequest, DatabaseResponse,
    ExtendedConsensusError,
};

#[derive(Debug)]
enum CachedAmount {
    Clear(u64),
    Commitment(EdwardsPoint),
}

impl CachedAmount {
    fn get_commitment(&self) -> EdwardsPoint {
        match self {
            CachedAmount::Commitment(commitment) => *commitment,
            // TODO: Setup a table with common amounts.
            CachedAmount::Clear(amt) => ED25519_BASEPOINT_POINT + H() * Scalar::from(*amt),
        }
    }
}

#[derive(Debug)]
struct CachedOutput {
    height: u64,
    time_lock: Timelock,
    key: CompressedEdwardsY,
    amount: CachedAmount,
}

#[derive(Debug)]
pub struct OutputCache {
    outputs: HashMap<u64, BTreeMap<u64, CachedOutput>>,
    amount_of_outputs: HashMap<u64, BTreeMap<u64, usize>>,
}

impl OutputCache {
    pub fn get_out(&self, amt: u64, idx: u64) -> Option<OutputOnChain> {
        let cached_out = self.outputs.get(&amt)?.get(&idx)?;

        Some(OutputOnChain {
            height: cached_out.height,
            time_lock: cached_out.time_lock,
            key: cached_out.key.decompress(),
            commitment: cached_out.amount.get_commitment(),
        })
    }

    pub fn outputs_in_cache_with_amount(&self, amt: u64, up_to_height: u64) -> usize {
        if amt == 0 {
            return 0;
        }

        let Some(map) = self.amount_of_outputs.get(&amt) else {
            return 0;
        };

        map.range(0..up_to_height).map(|(_, numb)| numb).sum()
    }

    pub async fn new_from_blocks<D: Database>(
        blocks: impl Iterator<Item = (&Block, &[Arc<TransactionVerificationData>])>,
        database: &mut D,
    ) -> Result<Self, ExtendedConsensusError> {
        let mut idx_needed = HashMap::new();

        for (block, txs) in blocks {
            for tx in once(&block.miner_tx).chain(txs.iter().map(|tx| &tx.tx)) {
                let is_rct = tx.prefix.version == 2;
                let is_miner = matches!(tx.prefix.inputs.as_slice(), &[Input::Gen(_)]);

                for (i, out) in tx.prefix.outputs.iter().enumerate() {
                    let amt = out.amount.unwrap_or(0);
                    // The amt this output will be stored under.
                    let amt_table_key = if is_rct { 0 } else { amt };

                    let amount_commitment = match (is_rct, is_miner) {
                        (true, false) => CachedAmount::Commitment(
                            *tx.rct_signatures.base.commitments.get(i).ok_or(
                                ConsensusError::Transaction(TransactionError::NonZeroOutputForV2),
                            )?,
                        ),
                        _ => CachedAmount::Clear(amt),
                    };
                    let output_to_cache = CachedOutput {
                        height: block.number().ok_or(ConsensusError::Block(
                            BlockError::MinerTxError(MinerTxError::InputNotOfTypeGen),
                        ))?,
                        time_lock: tx.prefix.timelock,
                        key: out.key,
                        amount: amount_commitment,
                    };

                    idx_needed
                        .entry(amt_table_key)
                        .or_insert_with(Vec::new)
                        .push(output_to_cache);
                }
            }
        }

        let DatabaseResponse::NumberOutputsWithAmount(numb_outs) = database
            .ready()
            .await?
            .call(DatabaseRequest::NumberOutputsWithAmount(
                idx_needed.keys().copied().collect(),
            ))
            .await?
        else {
            panic!("Database sent incorrect response!");
        };

        let mut outputs: HashMap<_, BTreeMap<_, _>> = HashMap::with_capacity(idx_needed.len() * 16);
        let mut amount_of_outputs: HashMap<_, BTreeMap<_, _>> = HashMap::new();

        for (amt_table_key, out) in idx_needed {
            let numb_outs = *numb_outs
                .get(&amt_table_key)
                .expect("DB did not return all results!");

            let mut height_to_amount = BTreeMap::new();

            outputs
                .entry(amt_table_key)
                .or_default()
                .extend(out.into_iter().enumerate().map(|(i, out)| {
                    *height_to_amount.entry(out.height).or_default() += 1_usize;

                    (u64::try_from(i + numb_outs).unwrap(), out)
                }));

            amount_of_outputs.insert(amt_table_key, height_to_amount);
        }

        Ok(OutputCache {
            outputs,
            amount_of_outputs,
        })
    }
}
