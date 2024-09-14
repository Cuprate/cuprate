//! TODO

use crate::block;

/// `MONEY_SUPPLY` - total number coins to be generated
pub const MONEY_SUPPLY: u64 = u64::MAX - 1;

/// TODO
pub const EMISSION_SPEED_FACTOR_PER_MINUTE: u64 = 20;

/// TODO
pub const FINAL_SUBSIDY_PER_MINUTE: u64 = 300_000_000_000; // 3 * pow(10, 11);

/// TODO
pub const CRYPTONOTE_DISPLAY_DECIMAL_POINT: u64 = 12;

/// COIN - number of smallest units in one coin
pub const COIN: u64 = 1000000000000; // pow(10, 12);

/// TODO
pub const FEE_PER_KB_OLD: u64 = 10000000000; // pow(10, 10);

/// TODO
pub const FEE_PER_KB: u64 = 2000000000; // 2 * pow(10, 9);

/// TODO
pub const FEE_PER_BYTE: u64 = 300000;

/// TODO
pub const DYNAMIC_FEE_PER_KB_BASE_FEE: u64 = 2000000000; // 2 * pow(10,9);

/// TODO
pub const DYNAMIC_FEE_PER_KB_BASE_BLOCK_REWARD: u64 = 10000000000000; // 10 * pow(10,12);

/// TODO
pub const DYNAMIC_FEE_PER_KB_BASE_FEE_V5: u64 = 2000000000
    * block::CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V2
    / block::CRYPTONOTE_BLOCK_GRANTED_FULL_REWARD_ZONE_V5;

/// TODO
pub const DYNAMIC_FEE_REFERENCE_TRANSACTION_WEIGHT: u64 = 3000;
