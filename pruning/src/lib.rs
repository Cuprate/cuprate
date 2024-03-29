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
//! match PruningSeed::decompress_p2p_rules(seed) {
//!     Ok(seed) => seed, // seed is valid
//!     Err(e) => panic!("seed is invalid")
//! };
//! ```
//!

use std::cmp::Ordering;

use thiserror::Error;

pub const CRYPTONOTE_MAX_BLOCK_HEIGHT: u64 = 500000000;
/// The default log stripes for Monero pruning.
pub const CRYPTONOTE_PRUNING_LOG_STRIPES: u32 = 3;
/// The amount of blocks that peers keep before another stripe starts storing blocks.
pub const CRYPTONOTE_PRUNING_STRIPE_SIZE: u64 = 4096;
/// The amount of blocks from the top of the chain that should not be pruned.
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
    #[error("The block height is greater than `CRYPTONOTE_MAX_BLOCK_HEIGHT`")]
    BlockHeightTooLarge,
    #[error("The blockchain height is greater than `CRYPTONOTE_MAX_BLOCK_HEIGHT`")]
    BlockChainHeightTooLarge,
    #[error("The calculated height is smaller than the block height entered")]
    CalculatedHeightSmallerThanEnteredBlock,
    #[error("The entered seed has incorrect log stripes")]
    SeedDoesNotHaveCorrectLogStripes,
}

/// A valid pruning seed for a Monero node.
///
/// A pruning seed tells nodes which blocks they should keep and which they should prune.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum PruningSeed {
    /// A peer with this seed is not pruned.
    NotPruned,
    /// A peer with this seed is pruned.
    Pruned(DecompressedPruningSeed),
}

impl PruningSeed {
    /// Creates a new [`PruningSeed::Pruned`] seed.
    ///
    /// See: [`DecompressedPruningSeed::new`]
    pub fn new_pruned(stripe: u32, log_stripes: u32) -> Result<Self, PruningError> {
        Ok(PruningSeed::Pruned(DecompressedPruningSeed::new(
            stripe,
            log_stripes,
        )?))
    }

    /// Attempts to decompress a raw pruning seed.
    ///
    /// An error means the pruning seed was invalid.
    pub fn decompress(seed: u32) -> Result<Self, PruningError> {
        Ok(DecompressedPruningSeed::decompress(seed)?
            .map(PruningSeed::Pruned)
            .unwrap_or(PruningSeed::NotPruned))
    }

    /// Decompresses the seed, performing the same checks as [`PruningSeed::decompress`] and some more according to
    /// Monero's p2p networks rules.
    ///
    /// The only added check currently is that `log_stripes` == 3.
    pub fn decompress_p2p_rules(seed: u32) -> Result<Self, PruningError> {
        let seed = Self::decompress(seed)?;

        if let Some(log_stripes) = seed.get_log_stripes() {
            if log_stripes != CRYPTONOTE_PRUNING_LOG_STRIPES {
                return Err(PruningError::LogStripesOutOfRange);
            }
        }

        Ok(seed)
    }

    /// Compresses this pruning seed to a u32.
    pub fn compress(&self) -> u32 {
        match self {
            PruningSeed::NotPruned => 0,
            PruningSeed::Pruned(seed) => seed.compress(),
        }
    }

    /// Returns the `log_stripes` for this seed, if this seed is pruned otherwise [`None`] is returned.
    pub fn get_log_stripes(&self) -> Option<u32> {
        match self {
            PruningSeed::NotPruned => None,
            PruningSeed::Pruned(seed) => Some(seed.log_stripes),
        }
    }

    /// Returns the `stripe` for this seed, if this seed is pruned otherwise [`None`] is returned.
    pub fn get_stripe(&self) -> Option<u32> {
        match self {
            PruningSeed::NotPruned => None,
            PruningSeed::Pruned(seed) => Some(seed.stripe),
        }
    }

    /// Returns if a peer with this pruning seed should have a non-pruned version of a block.
    pub fn has_full_block(&self, height: u64, blockchain_height: u64) -> bool {
        match self {
            PruningSeed::NotPruned => true,
            PruningSeed::Pruned(seed) => seed.has_full_block(height, blockchain_height),
        }
    }

    /// Gets the next pruned block for a given `block_height` and `blockchain_height`
    ///
    /// Each seed will store, in a cyclic manner, a portion of blocks while discarding
    /// the ones that are out of your stripe. This function is finding the next height
    /// for which a specific seed will start pruning blocks.
    ///
    /// This will return Ok(None) if the seed does no pruning or if there is no pruned block
    /// after this one.
    ///
    /// ### Errors
    ///
    /// This function will return an Error if the inputted `block_height` or
    /// `blockchain_height` is greater than [`CRYPTONOTE_MAX_BLOCK_HEIGHT`].
    ///
    /// This function will also error if `block_height` > `blockchain_height`
    pub fn get_next_pruned_block(
        &self,
        block_height: u64,
        blockchain_height: u64,
    ) -> Result<Option<u64>, PruningError> {
        Ok(match self {
            PruningSeed::NotPruned => None,
            PruningSeed::Pruned(seed) => {
                seed.get_next_pruned_block(block_height, blockchain_height)?
            }
        })
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
    /// `blockchain_height` is greater than [`CRYPTONOTE_MAX_BLOCK_HEIGHT`].
    ///
    /// This function will also error if `block_height` > `blockchain_height`
    ///
    pub fn get_next_unpruned_block(
        &self,
        block_height: u64,
        blockchain_height: u64,
    ) -> Result<u64, PruningError> {
        Ok(match self {
            PruningSeed::NotPruned => block_height,
            PruningSeed::Pruned(seed) => {
                seed.get_next_unpruned_block(block_height, blockchain_height)?
            }
        })
    }
}

impl PartialOrd<Self> for PruningSeed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PruningSeed {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Make sure pruning seeds storing more blocks are greater.
            (PruningSeed::NotPruned, PruningSeed::NotPruned) => Ordering::Equal,
            (PruningSeed::NotPruned, PruningSeed::Pruned(_)) => Ordering::Greater,
            (PruningSeed::Pruned(_), PruningSeed::NotPruned) => Ordering::Less,

            (PruningSeed::Pruned(seed1), PruningSeed::Pruned(seed2)) => seed1.cmp(seed2),
        }
    }
}

/// This represents a valid Monero pruning seed.
///
/// It does allow representations of pruning seeds that Monero's P2P network would not allow, i.e.
/// it does not restrict the seed to only have a `log_stripes` of 8.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct DecompressedPruningSeed {
    /// The amount of portions the blockchain is split into.
    log_stripes: u32,
    /// The specific portion this peer keeps.
    ///
    /// *MUST* be between 1..=2^log_stripes
    stripe: u32,
}

impl PartialOrd<Self> for DecompressedPruningSeed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DecompressedPruningSeed {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare the `log_stripes` first so peers which store more blocks are greater than peers
        // storing less.
        match self.log_stripes.cmp(&other.log_stripes) {
            Ordering::Equal => self.stripe.cmp(&other.stripe),
            ord => ord,
        }
    }
}

impl DecompressedPruningSeed {
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
    pub fn new(stripe: u32, log_stripes: u32) -> Result<Self, PruningError> {
        if log_stripes > PRUNING_SEED_LOG_STRIPES_MASK {
            Err(PruningError::LogStripesOutOfRange)
        } else if !(stripe > 0 && stripe <= (1 << log_stripes)) {
            Err(PruningError::StripeOutOfRange)
        } else {
            Ok(DecompressedPruningSeed {
                log_stripes,
                stripe,
            })
        }
    }

    /// Attempts to decompress a raw pruning seed.
    ///
    /// Will return Ok(None) if the pruning seed means no pruning.
    ///
    /// An error means the pruning seed was invalid.
    pub fn decompress(seed: u32) -> Result<Option<Self>, PruningError> {
        if seed == 0 {
            // No pruning.
            return Ok(None);
        }

        let log_stripes = (seed >> PRUNING_SEED_LOG_STRIPES_SHIFT) & PRUNING_SEED_LOG_STRIPES_MASK;
        let stripe = 1 + ((seed >> PRUNING_SEED_STRIPE_SHIFT) & PRUNING_SEED_STRIPE_MASK);

        if stripe > (1 << log_stripes) {
            return Err(PruningError::StripeOutOfRange);
        }

        Ok(Some(DecompressedPruningSeed {
            log_stripes,
            stripe,
        }))
    }

    /// Compresses the pruning seed into a u32.
    pub fn compress(&self) -> u32 {
        (self.log_stripes << PRUNING_SEED_LOG_STRIPES_SHIFT)
            | ((self.stripe - 1) << PRUNING_SEED_STRIPE_SHIFT)
    }

    /// Returns if a peer with this pruning seed should have a non-pruned version of a block.
    pub fn has_full_block(&self, height: u64, blockchain_height: u64) -> bool {
        let Some(block_stripe) =
            get_block_pruning_stripe(height, blockchain_height, self.log_stripes)
        else {
            return true;
        };

        self.stripe == block_stripe
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
    /// `blockchain_height` is greater than [`CRYPTONOTE_MAX_BLOCK_HEIGHT`].
    ///
    /// This function will also error if `block_height` > `blockchain_height`
    ///
    pub fn get_next_unpruned_block(
        &self,
        block_height: u64,
        blockchain_height: u64,
    ) -> Result<u64, PruningError> {
        if block_height > CRYPTONOTE_MAX_BLOCK_HEIGHT || block_height > blockchain_height {
            return Err(PruningError::BlockHeightTooLarge);
        }

        if blockchain_height > CRYPTONOTE_MAX_BLOCK_HEIGHT {
            return Err(PruningError::BlockChainHeightTooLarge);
        }

        if block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height {
            // If we are within `CRYPTONOTE_PRUNING_TIP_BLOCKS` of the chain we should
            // not prune blocks.
            return Ok(block_height);
        }

        let block_pruning_stripe = get_block_pruning_stripe(block_height, blockchain_height, self.log_stripes)
                .expect("We just checked if `block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height`");
        if self.stripe == block_pruning_stripe {
            // if we have the same stripe as a block that means we keep the block so
            // the entered block is the next un-pruned one.
            return Ok(block_height);
        }

        // cycles: how many times each seed has stored blocks so when all seeds have
        // stored blocks thats 1 cycle
        let cycles = (block_height / CRYPTONOTE_PRUNING_STRIPE_SIZE) >> self.log_stripes;
        // if our seed is before the blocks seed in a cycle that means we have already past our
        // seed this cycle and need to start the next
        let cycles_start = cycles
            + if self.stripe > block_pruning_stripe {
                0
            } else {
                1
            };

        // amt_of_cycles * blocks in a cycle + how many blocks through a cycles until the seed starts storing blocks
        let calculated_height = cycles_start * (CRYPTONOTE_PRUNING_STRIPE_SIZE << self.log_stripes)
            + (self.stripe as u64 - 1) * CRYPTONOTE_PRUNING_STRIPE_SIZE;

        if calculated_height + CRYPTONOTE_PRUNING_TIP_BLOCKS > blockchain_height {
            // if our calculated height is greater than the amount of tip blocks then the start of the tip blocks will be the next un-pruned
            Ok(blockchain_height.saturating_sub(CRYPTONOTE_PRUNING_TIP_BLOCKS))
        } else if calculated_height < block_height {
            Err(PruningError::CalculatedHeightSmallerThanEnteredBlock)
        } else {
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
    /// `blockchain_height` is greater than [`CRYPTONOTE_MAX_BLOCK_HEIGHT`].
    ///
    /// This function will also error if `block_height` > `blockchain_height`
    ///
    pub fn get_next_pruned_block(
        &self,
        block_height: u64,
        blockchain_height: u64,
    ) -> Result<Option<u64>, PruningError> {
        if block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height {
            // If we are within `CRYPTONOTE_PRUNING_TIP_BLOCKS` of the chain we should
            // not prune blocks.
            return Ok(None);
        }

        let block_pruning_stripe = get_block_pruning_stripe(block_height, blockchain_height, self.log_stripes)
            .expect("We just checked if `block_height + CRYPTONOTE_PRUNING_TIP_BLOCKS >= blockchain_height`");
        if self.stripe != block_pruning_stripe {
            // if our stripe != the blocks stripe that means we prune that block
            return Ok(Some(block_height));
        }

        // We can get the end of our "non-pruning" cycle by getting the next stripe's first un-pruned block height.
        // So we calculate the next un-pruned block for the next stripe and return it as our next pruned block
        let next_stripe = 1 + (self.stripe & ((1 << self.log_stripes) - 1));
        let seed = DecompressedPruningSeed::new(next_stripe, self.log_stripes)
            .expect("We just made sure this stripe is in range for this log_stripe");

        let calculated_height = seed.get_next_unpruned_block(block_height, blockchain_height)?;

        if calculated_height + CRYPTONOTE_PRUNING_TIP_BLOCKS > blockchain_height {
            // If the calculated height is in tip blocks then there is no next block to prune
            Ok(None)
        } else {
            Ok(Some(calculated_height))
        }
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
        let possible_stripes = 1..=(1 << CRYPTONOTE_PRUNING_LOG_STRIPES);
        possible_stripes
            .map(|stripe| PruningSeed::new_pruned(stripe, CRYPTONOTE_PRUNING_LOG_STRIPES).unwrap())
            .collect()
    }

    #[test]
    fn from_u32_for_pruning_seed() {
        let good_seeds = 384..=391;
        for seed in good_seeds {
            assert!(PruningSeed::decompress(seed).is_ok());
        }
        let bad_seeds = [383, 392];
        for seed in bad_seeds {
            assert!(PruningSeed::decompress(seed).is_err());
        }
    }

    #[test]
    fn make_invalid_pruning_seeds() {
        let invalid_stripes = [0, (1 << CRYPTONOTE_PRUNING_LOG_STRIPES) + 1];

        for stripe in invalid_stripes {
            assert!(PruningSeed::new_pruned(stripe, CRYPTONOTE_PRUNING_LOG_STRIPES).is_err());
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

        let zero_seed = PruningSeed::NotPruned;

        assert_eq!(
            zero_seed.get_next_unpruned_block(33443, 5565445).unwrap(),
            33443
        );

        let seed = PruningSeed::decompress(384).unwrap();

        // the next unpruned block is the first tip block
        assert_eq!(seed.get_next_unpruned_block(5000, 11000).unwrap(), 5500)
    }

    #[test]
    fn next_pruned_block() {
        let all_valid_seeds = make_all_pruning_seeds();
        let blockchain_height = 76437863;

        for seed in all_valid_seeds.iter().skip(1) {
            assert_eq!(
                seed.get_next_pruned_block(0, blockchain_height)
                    .unwrap()
                    .unwrap(),
                0
            )
        }

        for (i, seed) in all_valid_seeds.iter().enumerate() {
            assert_eq!(
                seed.get_next_pruned_block((i as u64 + 1) * 4096, blockchain_height)
                    .unwrap()
                    .unwrap(),
                (i as u64 + 1) * 4096
            )
        }

        for (i, seed) in all_valid_seeds.iter().enumerate() {
            assert_eq!(
                seed.get_next_pruned_block((i as u64 + 8) * 4096, blockchain_height)
                    .unwrap()
                    .unwrap(),
                (i as u64 + 9) * 4096
            )
        }

        for seed in all_valid_seeds.iter() {
            assert_eq!(
                seed.get_next_pruned_block(76437863 - 1, blockchain_height)
                    .unwrap(),
                None
            )
        }

        let zero_seed = PruningSeed::NotPruned;

        assert_eq!(
            zero_seed.get_next_pruned_block(33443, 5565445).unwrap(),
            None
        );

        let seed = PruningSeed::decompress(384).unwrap();

        // there is no next pruned block
        assert_eq!(seed.get_next_pruned_block(5000, 10000).unwrap(), None)
    }
}
