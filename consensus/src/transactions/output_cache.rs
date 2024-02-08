use std::{
    collections::{BTreeMap, HashMap},
    iter::once,
    sync::{Arc, OnceLock},
};

use curve25519_dalek::{
    constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY, EdwardsPoint, Scalar,
};
use monero_consensus::{blocks::BlockError, miner_tx::MinerTxError, ConsensusError};
use monero_serai::{
    block::Block,
    transaction::{Input, Timelock},
    H,
};
use tower::ServiceExt;

use monero_consensus::transactions::{OutputOnChain, TransactionError};

use crate::transactions::TransactionVerificationData;
use crate::{Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError};

#[derive(Debug)]
enum CachedAmount<'a> {
    Clear(u64),
    Commitment(&'a EdwardsPoint),
}

impl<'a> CachedAmount<'a> {
    fn get_commitment(&self) -> EdwardsPoint {
        match self {
            CachedAmount::Commitment(commitment) => **commitment,
            // TODO: Setup a table with common amounts.
            CachedAmount::Clear(amt) => ED25519_BASEPOINT_POINT + H() * Scalar::from(*amt),
        }
    }
}

#[derive(Debug)]
struct CachedOutput<'a> {
    height: u64,
    time_lock: &'a Timelock,
    key: &'a CompressedEdwardsY,
    amount: CachedAmount<'a>,

    cached_created: OnceLock<OutputOnChain>,
}

#[derive(Debug)]
pub struct OutputCache<'a>(HashMap<u64, BTreeMap<u64, CachedOutput<'a>>>);

impl<'a> OutputCache<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        OutputCache(HashMap::new())
    }

    pub fn get_out(&self, amt: u64, idx: u64) -> Option<&OutputOnChain> {
        let cached_out = self.0.get(&amt)?.get(&idx)?;

        Some(cached_out.cached_created.get_or_init(|| OutputOnChain {
            height: cached_out.height,
            time_lock: *cached_out.time_lock,
            key: cached_out.key.decompress(),
            commitment: cached_out.amount.get_commitment(),
        }))
    }

    pub async fn extend_from_block<'b: 'a, D: Database>(
        &mut self,
        blocks: impl Iterator<Item = (&'b Block, &'b [Arc<TransactionVerificationData>])> + 'b,
        database: &mut D,
    ) -> Result<(), ExtendedConsensusError> {
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
                            tx.rct_signatures.base.commitments.get(i).ok_or(
                                ConsensusError::Transaction(TransactionError::NonZeroOutputForV2),
                            )?,
                        ),
                        _ => CachedAmount::Clear(amt),
                    };
                    let output_to_cache = CachedOutput {
                        height: block.number().ok_or(ConsensusError::Block(
                            BlockError::MinerTxError(MinerTxError::InputNotOfTypeGen),
                        ))?,
                        time_lock: &tx.prefix.timelock,
                        key: &out.key,
                        amount: amount_commitment,

                        cached_created: OnceLock::new(),
                    };

                    let Some(amt_table) = self.0.get_mut(&amt_table_key) else {
                        idx_needed
                            .entry(amt_table_key)
                            .or_insert_with(Vec::new)
                            .push(output_to_cache);
                        continue;
                    };

                    let top_idx = *amt_table.last_key_value().unwrap().0;
                    amt_table.insert(top_idx + 1, output_to_cache);
                }
            }
        }

        if idx_needed.is_empty() {
            return Ok(());
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

        for (amt_table_key, out) in idx_needed {
            let numb_outs = *numb_outs
                .get(&amt_table_key)
                .expect("DB did not return all results!");

            self.0.entry(amt_table_key).or_default().extend(
                out.into_iter()
                    .enumerate()
                    .map(|(i, out)| (u64::try_from(i + numb_outs).unwrap(), out)),
            )
        }

        Ok(())
    }
}
