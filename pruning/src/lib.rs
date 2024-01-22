//! # Pruning Mechanism for Monero
//!
//! This crate provides an implementation of the pruning mechanism used in Monero.
//! The main data structure, `PruningSeed`, encapsulates the logic for creating and manipulating pruning seeds,
//! which determine the set of blocks to be pruned from the blockchain.
//!
//! `PruningSeed` also contains a method for checking if a pruning seed is valid for Monero rules (must only be
//! split into 8 parts):
//!
//! ```rust
//! use monero_pruning::PruningSeed;
//!
//! let seed: u32 = 386; // the seed you want to check is valid
//! match PruningSeed::try_from(seed) {
//!     Ok(seed) => seed, // seed is valid
//!     Err(e) => panic!("seed is invalid")
//! };
//! ```
//!

use std::cmp::Ordering;

use thiserror::Error;

pub const CRYPTONOTE_MAX_BLOCK_NUMBER: u64 = 500000000;

pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u32 = 3;
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: u64 = 4096;
pub const CRYPTONOTE_PRUNING_TIP_BLOCKS: u64 = 5500;

const PRUNING_SEED_LOG_STRIPES_SHIFT: u32 = 7;
const PRUNING_SEED_STRIPE_SHIFT: u32 = 0;
const PRUNING_SEED_LOG_STRIPES_MASK: u32 = 0x7;
const PRUNING_SEED_STRIPE_MASK: u32 = 127;

#[derive(Debug, Error)]
pub enum PruningError {
    #[error("log_stripes is out of range")]
    LogStripesOutOfRange,
    #[error("Stripe is out of range")]
    StripeOutOfRange,
    #[error("The block height is greater than `CRYPTONOTE_MAX_BLOCK_NUMBER`")]
    BlockHeightTooLarge,
    #[error("The blockchain height is greater than `CRYPTONOTE_MAX_BLOCK_NUMBER`")]
    BlockChainHeightTooLarge,
    #[error("The calculated height is smaller than the block height entered")]
    CalculatedHeightSmallerThanEnteredBlock,
    #[error("The entered seed has incorrect log stripes")]
    SeedDoesNotHaveCorrectLogStripes,
}

/// A Monero pruning seed which has methods to get the next pruned/ unpruned block.
///
// Internally we use an Option<u32> to represent if a pruning seed is 0 (None)which means
// no pruning will take place.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct PruningSeed(Option<u32>);

impl PartialOrd for PruningSeed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PruningSeed {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.get_log_stripes(), other.get_log_stripes()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(stripe_s), Some(stripe_o)) => match stripe_s.cmp(&stripe_o) {
                Ordering::Equal => self.get_stripe().unwrap().cmp(&other.get_stripe().unwrap()),
                ordering => ordering,
            },
        }
    }
}

impl PruningSeed {
    /// Creates a new pruning seed from a `stripe` and `log_stripes`
    ///
    /// ### What is a `stripe`
    ///
    /// A stripe is the part of the blockchain this peer will keep.
    ///  
    /// Monero, when pruning, will split the blockchain into multiple
    /// "stripes", that amount is currently 8 and each pruned peer
    /// will keep one of those 8 stripes.
    ///
    /// ### What is `log_stripes`
    ///
    /// `log_stripes` is log2 of the amount of stripes used.
    ///
    ///  For Monero, currently, that amount is 8 so `log_stripes` will
    ///  be 3.
    ///
    /// ---------------------------------------------------------------
    ///
    /// *note this function allows you to make invalid seeds, this is done
    /// to allow the specifics of pruning to change in the future. To make
    /// a valid seed you currently MUST pass in a number 1 to 8 for `stripe`
    /// and 3 for `log_stripes`.*
    ///
    pub fn new(stripe: u32, log_stripes: u32) -> Result<PruningSeed, PruningError> {
        if log_stripes > PRUNING_SEED_LOG_STRIPES_MASK {
            Err(PruningError::LogStripesOutOfRange)
        } else if !(stripe > 0 && stripe <= (1 << log_stripes)) {
            Err(PruningError::StripeOutOfRange)
        } else {
            Ok(PruningSeed(Some(
                (log_stripes << PRUNING_SEED_LOG_STRIPES_SHIFT)
                    | ((stripe - 1) << PRUNING_SEED_STRIPE_SHIFT),
            )))
        }
    }

    // Gets log2 of the total amount of stripes this seed is using.
    fn get_log_stripes(&self) -> Option<u32> {
        let seed: u32 = self.0?;
        Some((seed >> PRUNING_SEED_LOG_STRIPES_SHIFT) & PRUNING_SEED_LOG_STRIPES_MASK)
    }

    // Gets the specific stripe of this seed.
    fn get_stripe(&self) -> Option<u32> {
        let seed: u32 = self.0?;
        Some(1 + ((seed >> PRUNING_SEED_STRIPE_SHIFT) & PRUNING_SEED_STRIPE_MASK))
    }

    /// Gets the next unpruned block for a given `block_height` and `blockchain_height`
    ///
    /// Each seed will store, in a cyclic manner, a portion of blocks while discarding
    /// the ones that are out of your stripe. This function is finding the next height
    /// for which a specific seed will start storing blocks.
    ///
    /// ### Errors
    ///
    /// This function will return an Error if the inputted `block_height` or
    /// `blockchain_height` is greater than [`CRYPTONOTE_MAX_BLOCK_NUMBER`].
    ///
    /// This function will also error if `block_height` > `blockchain_height`
    ///
    pub fn get_next_unpruned_block(
        &self,
        block_height: u64,
        blockchain_height: u64,
    ) -> Result<u64, PruningError> {
        if block_height > CRYPTONOTE_MAX_BLOCK_NUMBER || block_height > blockchain_height {
            Err(PruningError::BlockHeightTooLarge)
        } else if blockchain_height > CRYPTONOTE_MAX_BLOCK_NUMBER {
            Err(PruningError::BlockChainHeightTooLarge)
        } else {
            let Some(seed_stripe) = self.get_stripe() else {
                // If the `get_stripe` returns None that means no pruning so the next
                // unpruned block is the one inputted.
                return Ok(block_height);
            };
            if block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height {
                // If we are within `CRYPTONOTE_PRUNING_TIP_BLOCKS` of the chain we should
                // not prune blocks.
                return Ok(block_height);
            }
            let seed_log_stripes = self
                .get_log_stripes()
                .unwrap_or(CRYPTONOTE_PRUNING_LOG_STRIPES);
            let block_pruning_stripe = get_block_pruning_stripe(block_height, blockchain_height, seed_log_stripes)
                .expect("We just checked if `block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height`");
            if seed_stripe == block_pruning_stripe {
                // if we have the same stripe as a block that means we keep the block so
                // the entered block is the next un-pruned one.
                return Ok(block_height);
            }

            // cycles: how many times each seed has stored blocks so when all seeds have
            // stored blocks thats 1 cycle
            let cycles = (block_height / CRYPTONOTE_PRUNING_STRIPE_SIZE) >> seed_log_stripes;
            // if our seed is before the blocks seed in a cycle that means we have already past our
            // seed this cycle and need to start the next
            let cycles_start = cycles
                + if seed_stripe > block_pruning_stripe {
                    0
                } else {
                    1
                };

            // amt_of_cycles * blocks in a cycle + how many blocks through a cycles until the seed starts storing blocks
            let calculated_height = cycles_start
                * (CRYPTONOTE_PRUNING_STRIPE_SIZE << seed_log_stripes)
                + (seed_stripe as u64 - 1) * CRYPTONOTE_PRUNING_STRIPE_SIZE;
            if calculated_height + CRYPTONOTE_PRUNING_TIP_BLOCKS > blockchain_height {
                // if our calculated height is greater than the amount of tip blocks the the start of the tip blocks will be the next un-pruned
                return Ok(blockchain_height.saturating_sub(CRYPTONOTE_PRUNING_TIP_BLOCKS));
            }
            if calculated_height < block_height {
                return Err(PruningError::CalculatedHeightSmallerThanEnteredBlock);
            }
            Ok(calculated_height)
        }
    }

    /// Gets the next pruned block for a given `block_height` and `blockchain_height`
    ///
    /// Each seed will store, in a cyclic manner, a portion of blocks while discarding
    /// the ones that are out of your stripe. This function is finding the next height
    /// for which a specific seed will start pruning blocks.
    ///
    /// ### Errors
    ///
    /// This function will return an Error if the inputted `block_height` or
    /// `blockchain_height` is greater than [`CRYPTONOTE_MAX_BLOCK_NUMBER`].
    ///
    /// This function will also error if `block_height` > `blockchain_height`
    ///
    pub fn get_next_pruned_block(
        &self,
        block_height: u64,
        blockchain_height: u64,
    ) -> Result<u64, PruningError> {
        let Some(seed_stripe) = self.get_stripe() else {
            // If the `get_stripe` returns None that means no pruning so the next
            // pruned block is nonexistent so we return the blockchain_height.
            return Ok(blockchain_height);
        };
        if block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height {
            // If we are within `CRYPTONOTE_PRUNING_TIP_BLOCKS` of the chain we should
            // not prune blocks.
            return Ok(blockchain_height);
        }
        let seed_log_stripes = self
            .get_log_stripes()
            .unwrap_or(CRYPTONOTE_PRUNING_LOG_STRIPES);
        let block_pruning_stripe = get_block_pruning_stripe(block_height, blockchain_height, seed_log_stripes)
            .expect("We just checked if `block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height`");
        if seed_stripe != block_pruning_stripe {
            // if our stripe != the blocks stripe that means we prune that block
            return Ok(block_height);
        }

        // We can get the end of our "non-pruning" cycle by getting the next stripe's after us first un-pruned block height
        // so we calculate the next un-pruned block for the next stripe and return it as our next pruned block
        let next_stripe = (1 + seed_log_stripes) & ((1 << seed_log_stripes) - 1);
        let seed = PruningSeed::new(next_stripe, seed_log_stripes)?;
        seed.get_next_unpruned_block(block_height, blockchain_height)
    }
}

impl TryFrom<u32> for PruningSeed {
    type Error = PruningError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(PruningSeed(None))
        } else {
            let seed = Self(Some(value));
            let log_stripes = seed.get_log_stripes().expect("This will only return None if the inner value is None which will only happen if the seed is 0 but we checked for that");
            if log_stripes != CRYPTONOTE_PRUNING_LOG_STRIPES {
                return Err(PruningError::SeedDoesNotHaveCorrectLogStripes);
            }
            if seed.get_stripe().expect("same as above") > (1 << log_stripes) {
                return Err(PruningError::StripeOutOfRange);
            }
            Ok(seed)
        }
    }
}

impl From<PruningSeed> for u32 {
    fn from(value: PruningSeed) -> Self {
        value.0.unwrap_or(0)
    }
}

fn get_block_pruning_stripe(
    block_height: u64,
    blockchain_height: u64,
    log_stripe: u32,
) -> Option<u32> {
    if block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height {
        None
    } else {
        Some(
            (((block_height / CRYPTONOTE_PRUNING_STRIPE_SIZE) & ((1 << log_stripe) as u64 - 1)) + 1)
                as u32, // it's trivial to prove it's ok to us `as` here
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_all_pruning_seeds() -> Vec<PruningSeed> {
        let possible_stripes = 1..(1 << CRYPTONOTE_PRUNING_LOG_STRIPES);
        possible_stripes
            .map(|stripe| PruningSeed::new(stripe, CRYPTONOTE_PRUNING_LOG_STRIPES).unwrap())
            .collect()
    }

    #[test]
    fn from_u32_for_pruning_seed() {
        let good_seeds = 384..=391;
        for seed in good_seeds {
            assert!(PruningSeed::try_from(seed).is_ok());
        }
        let bad_seeds = [383, 392];
        for seed in bad_seeds {
            assert!(PruningSeed::try_from(seed).is_err());
        }
    }

    #[test]
    fn make_invalid_pruning_seeds() {
        let invalid_stripes = [0, (1 << CRYPTONOTE_PRUNING_LOG_STRIPES) + 1];

        for stripe in invalid_stripes {
            assert!(PruningSeed::new(stripe, CRYPTONOTE_PRUNING_LOG_STRIPES).is_err());
        }
    }

    #[test]
    fn get_pruning_log_stripe() {
        let all_valid_seeds = make_all_pruning_seeds();
        for seed in all_valid_seeds.iter() {
            assert_eq!(seed.get_log_stripes().unwrap(), 3)
        }
    }

    #[test]
    fn get_pruning_stripe() {
        let all_valid_seeds = make_all_pruning_seeds();
        for (i, seed) in all_valid_seeds.iter().enumerate() {
            assert_eq!(seed.get_stripe().unwrap(), i as u32 + 1)
        }
    }

    #[test]
    fn blocks_pruning_stripe() {
        let blockchain_height = 76437863;

        for i in 0_u32..8 {
            assert_eq!(
                get_block_pruning_stripe(
                    (i * 4096) as u64,
                    blockchain_height,
                    CRYPTONOTE_PRUNING_LOG_STRIPES
                )
                .unwrap(),
                i + 1
            );
        }

        for i in 0_u32..8 {
            assert_eq!(
                get_block_pruning_stripe(
                    32768 + (i * 4096) as u64,
                    blockchain_height,
                    CRYPTONOTE_PRUNING_LOG_STRIPES
                )
                .unwrap(),
                i + 1
            );
        }

        for i in 1_u32..8 {
            assert_eq!(
                get_block_pruning_stripe(
                    32767 + (i * 4096) as u64,
                    blockchain_height,
                    CRYPTONOTE_PRUNING_LOG_STRIPES
                )
                .unwrap(),
                i
            );
        }

        // Block shouldn't be pruned
        assert!(get_block_pruning_stripe(
            blockchain_height - 5500,
            blockchain_height,
            CRYPTONOTE_PRUNING_LOG_STRIPES
        )
        .is_none());
    }

    #[test]
    fn next_unpruned_block() {
        let all_valid_seeds = make_all_pruning_seeds();
        let blockchain_height = 76437863;

        for (i, seed) in all_valid_seeds.iter().enumerate() {
            assert_eq!(
                seed.get_next_unpruned_block(0, blockchain_height).unwrap(),
                i as u64 * 4096
            )
        }

        for (i, seed) in all_valid_seeds.iter().enumerate() {
            assert_eq!(
                seed.get_next_unpruned_block((i as u64 + 1) * 4096, blockchain_height)
                    .unwrap(),
                i as u64 * 4096 + 32768
            )
        }

        for (i, seed) in all_valid_seeds.iter().enumerate() {
            assert_eq!(
                seed.get_next_unpruned_block((i as u64 + 8) * 4096, blockchain_height)
                    .unwrap(),
                i as u64 * 4096 + 32768
            )
        }

        for seed in all_valid_seeds.iter() {
            assert_eq!(
                seed.get_next_unpruned_block(76437863 - 1, blockchain_height)
                    .unwrap(),
                76437863 - 1
            )
        }

        let zero_seed = PruningSeed(None);

        assert_eq!(
            zero_seed.get_next_unpruned_block(33443, 5565445).unwrap(),
            33443
        );

        let seed = PruningSeed(Some(384));

        // the next unpruned block is the first tip block
        assert_eq!(seed.get_next_unpruned_block(5000, 11000).unwrap(), 5500)
    }

    // TODO: next_pruned_block
}
