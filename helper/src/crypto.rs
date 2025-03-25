//! Crypto related functions and runtime initialized constants

//---------------------------------------------------------------------------------------------------- Use
use std::sync::LazyLock;

use curve25519_dalek::{
    constants::{ED25519_BASEPOINT_COMPRESSED, ED25519_BASEPOINT_POINT},
    edwards::CompressedEdwardsY,
    edwards::VartimeEdwardsPrecomputation,
    traits::VartimePrecomputedMultiscalarMul,
    Scalar,
};
use monero_serai::generators::H;

//---------------------------------------------------------------------------------------------------- Pre-computation

/// This is the decomposed amount table containing the mandatory Pre-RCT amounts. It is used to pre-compute 
/// zero commitments at runtime.
/// 
/// Defined at:
/// - <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/src/ringct/rctOps.cpp#L44>
#[rustfmt::skip]
pub const ZERO_COMMITMENT_DECOMPOSED_AMOUNT: [u64; 172] = [
    1,                   2,                   3,                   4,                   5,                   6,                   7,                   8,                   9,
    10,                  20,                  30,                  40,                  50,                  60,                  70,                  80,                  90,
    100,                 200,                 300,                 400,                 500,                 600,                 700,                 800,                 900,
    1000,                2000,                3000,                4000,                5000,                6000,                7000,                8000,                9000,
    10000,               20000,               30000,               40000,               50000,               60000,               70000,               80000,               90000,
    100000,              200000,              300000,              400000,              500000,              600000,              700000,              800000,              900000,
    1000000,             2000000,             3000000,             4000000,             5000000,             6000000,             7000000,             8000000,             9000000,
    10000000,            20000000,            30000000,            40000000,            50000000,            60000000,            70000000,            80000000,            90000000,
    100000000,           200000000,           300000000,           400000000,           500000000,           600000000,           700000000,           800000000,           900000000,
    1000000000,          2000000000,          3000000000,          4000000000,          5000000000,          6000000000,          7000000000,          8000000000,          9000000000,
    10000000000,         20000000000,         30000000000,         40000000000,         50000000000,         60000000000,         70000000000,         80000000000,         90000000000,
    100000000000,        200000000000,        300000000000,        400000000000,        500000000000,        600000000000,        700000000000,        800000000000,        900000000000,
    1000000000000,       2000000000000,       3000000000000,       4000000000000,       5000000000000,       6000000000000,       7000000000000,       8000000000000,       9000000000000,
    10000000000000,      20000000000000,      30000000000000,      40000000000000,      50000000000000,      60000000000000,      70000000000000,      80000000000000,      90000000000000,
    100000000000000,     200000000000000,     300000000000000,     400000000000000,     500000000000000,     600000000000000,     700000000000000,     800000000000000,     900000000000000,
    1000000000000000,    2000000000000000,    3000000000000000,    4000000000000000,    5000000000000000,    6000000000000000,    7000000000000000,    8000000000000000,    9000000000000000,
    10000000000000000,   20000000000000000,   30000000000000000,   40000000000000000,   50000000000000000,   60000000000000000,   70000000000000000,   80000000000000000,   90000000000000000,
    100000000000000000,  200000000000000000,  300000000000000000,  400000000000000000,  500000000000000000,  600000000000000000,  700000000000000000,  800000000000000000,  900000000000000000,
    1000000000000000000, 2000000000000000000, 3000000000000000000, 4000000000000000000, 5000000000000000000, 6000000000000000000, 7000000000000000000, 8000000000000000000, 9000000000000000000,
    10000000000000000000
];

/// Runtime initialized [`H`] generator.
static H_PRECOMP: LazyLock<VartimeEdwardsPrecomputation> =
    LazyLock::new(|| VartimeEdwardsPrecomputation::new([*H, ED25519_BASEPOINT_POINT]));

/// Runtime initialized zero commitment lookup table
///
/// # Invariant
/// This function assumes that the [`ZERO_COMMITMENT_DECOMPOSED_AMOUNT`]
/// table is sorted.
pub static ZERO_COMMITMENT_LOOKUP_TABLE: LazyLock<[CompressedEdwardsY; 172]> =
    LazyLock::new(|| {
        let mut lookup_table: [CompressedEdwardsY; 172] = [ED25519_BASEPOINT_COMPRESSED; 172];

        for (i, amount) in ZERO_COMMITMENT_DECOMPOSED_AMOUNT.into_iter().enumerate() {
            lookup_table[i] = (ED25519_BASEPOINT_POINT + *H * Scalar::from(amount)).compress();
        }

        lookup_table
    });

//---------------------------------------------------------------------------------------------------- Free functions

/// This function computes the zero commitment given a specific amount.
///
/// It will first attempt to lookup into the table of known Pre-RCT value.
/// Compute it otherwise.
#[expect(clippy::cast_possible_truncation)]
pub fn compute_zero_commitment(amount: u64) -> CompressedEdwardsY {
    // OPTIMIZATION: Unlike monerod which execute a linear search across its lookup
    // table (O(n)). Cuprate is making use of an arithmetic based constant time
    // version (O(1)). It has been benchmarked in both hit and miss scenarios against
    // a binary search lookup (O(log2(n))). To understand the following algorithm it
    // is important to observe the pattern that follows the values of
    // [`ZERO_COMMITMENT_DECOMPOSED_AMOUNT`].

    // First obtain the logarithm base 10 of the amount. and extend it back to obtain
    // the amount without its most significant digit.
    let Some(log) = amount.checked_ilog10() else {
        // amount = 0 so H component is 0.
        return ED25519_BASEPOINT_COMPRESSED;
    };
    let div = 10_u64.pow(log);

    // Extract the most significant digit.
    let most_significant_digit = amount / div;

    // If the *rounded* version is different than the exact amount. Then
    // there aren't only trailing zeroes behind the most significant digit.
    // The amount is not part of the table and can calculated apart.
    if most_significant_digit * div != amount {
        return H_PRECOMP
            .vartime_multiscalar_mul([Scalar::from(amount), Scalar::ONE])
            .compress();
    }

    // Calculating the index back by progressing within the powers of 10.
    // The index of the first value in the cached amount's row.
    let row_start = u64::from(log) * 9;
    // The index of the cached amount
    let index = (most_significant_digit - 1 + row_start) as usize;

    ZERO_COMMITMENT_LOOKUP_TABLE[index]
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use curve25519_dalek::{traits::VartimePrecomputedMultiscalarMul, Scalar};

    use crate::crypto::{compute_zero_commitment, H_PRECOMP, ZERO_COMMITMENT_DECOMPOSED_AMOUNT};

    #[test]
    /// Compare the output of `compute_zero_commitment` for all
    /// preRCT decomposed amounts against their actual computation.
    ///
    /// Assert that the lookup table returns the correct commitments
    fn compare_lookup_with_computation() {
        for amount in ZERO_COMMITMENT_DECOMPOSED_AMOUNT {
            let commitment = H_PRECOMP.vartime_multiscalar_mul([Scalar::from(amount), Scalar::ONE]);
            assert_eq!(
                commitment,
                compute_zero_commitment(amount).decompress().unwrap()
            );
        }
    }
}
