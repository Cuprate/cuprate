use crate::hardforks::HardFork;

const MONEY_SUPPLY: u64 = u64::MAX;
const MINIMUM_REWARD_PER_MIN: u64 = 3 * 10_u64.pow(11);

fn calculate_base_reward(already_generated_coins: u64, hf: &HardFork) -> u64 {
    let target_mins = hf.block_time().as_secs() / 60;
    let emission_speed_factor = 20 - (target_mins - 1);
    ((MONEY_SUPPLY - already_generated_coins) >> emission_speed_factor)
        .max(MINIMUM_REWARD_PER_MIN * target_mins)
}

pub fn calculate_block_reward(
    block_weight: u64,
    effective_median_bw: u64,
    already_generated_coins: u64,
    hf: &HardFork,
) -> u64 {
    let base_reward = calculate_base_reward(already_generated_coins, hf);

    let multiplicand = (2 * effective_median_bw - block_weight) * block_weight;
    let effective_median_bw: u128 = effective_median_bw.into();

    ((mul_128(base_reward, multiplicand) / effective_median_bw) / effective_median_bw)
        .try_into()
        .unwrap()
}

fn mul_128(a: u64, b: u64) -> u128 {
    a as u128 * b as u128
}
