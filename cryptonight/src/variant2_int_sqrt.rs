/// This file translates the C header:
/// https://github.com/monero-project/monero/blob/v0.18.3.3/src/crypto/variant2_int_sqrt.h

/*
#[inline]
fn variant2_integer_math_sqrt_step_fp64(sqrt_input: f64) -> f64 {
    sqrt_input.sqrt() * 2.0 - 8589934592.0
}

#[inline]
fn variant2_integer_math_sqrt_step_ref(sqrt_input: u64) -> u32 {
    integer_square_root_v2(sqrt_input)
}

 */


/*
    VARIANT2_INTEGER_MATH_SQRT_FIXUP checks that "r" is an integer part of "sqrt(2^64 + sqrt_input) * 2 - 2^33" and adds or subtracts 1 if needed
    It's hard to understand how it works, so here is a full calculation of formulas used in VARIANT2_INTEGER_MATH_SQRT_FIXUP

    The following inequalities must hold for r if it's an integer part of "sqrt(2^64 + sqrt_input) * 2 - 2^33":
    1) r <= sqrt(2^64 + sqrt_input) * 2 - 2^33
    2) r + 1 > sqrt(2^64 + sqrt_input) * 2 - 2^33

    We need to check them using only unsigned integer arithmetic to avoid rounding errors and undefined behavior

    First inequality: r <= sqrt(2^64 + sqrt_input) * 2 - 2^33
    -----------------------------------------------------------------------------------
    r <= sqrt(2^64 + sqrt_input) * 2 - 2^33
    r + 2^33 <= sqrt(2^64 + sqrt_input) * 2
    r/2 + 2^32 <= sqrt(2^64 + sqrt_input)
    (r/2 + 2^32)^2 <= 2^64 + sqrt_input

    Rewrite r as r = s * 2 + b (s = trunc(r/2), b is 0 or 1)

    ((s*2+b)/2 + 2^32)^2 <= 2^64 + sqrt_input
    (s*2+b)^2/4 + 2*2^32*(s*2+b)/2 + 2^64 <= 2^64 + sqrt_input
    (s*2+b)^2/4 + 2*2^32*(s*2+b)/2 <= sqrt_input
    (s*2+b)^2/4 + 2^32*r <= sqrt_input
    (s^2*4+2*s*2*b+b^2)/4 + 2^32*r <= sqrt_input
    s^2+s*b+b^2/4 + 2^32*r <= sqrt_input
    s*(s+b) + b^2/4 + 2^32*r <= sqrt_input

    Let r2 = s*(s+b) + r*2^32
    r2 + b^2/4 <= sqrt_input

    If this inequality doesn't hold, then we must decrement r: IF "r2 + b^2/4 > sqrt_input" THEN r = r - 1

    b can be 0 or 1
    If b is 0 then we need to compare "r2 > sqrt_input"
    If b is 1 then b^2/4 = 0.25, so we need to compare "r2 + 0.25 > sqrt_input"
    Since both r2 and sqrt_input are integers, we can safely replace it with "r2 + 1 > sqrt_input"
    -----------------------------------------------------------------------------------
    Both cases can be merged to a single expression "r2 + b > sqrt_input"
    -----------------------------------------------------------------------------------
    There will be no overflow when calculating "r2 + b", so it's safe to compare with sqrt_input:
    r2 + b = s*(s+b) + r*2^32 + b
    The largest value s, b and r can have is s = 1779033703, b = 1, r = 3558067407 when sqrt_input = 2^64 - 1
    r2 + b <= 1779033703*1779033704 + 3558067407*2^32 + 1 = 18446744068217447385 < 2^64

    Second inequality: r + 1 > sqrt(2^64 + sqrt_input) * 2 - 2^33
    -----------------------------------------------------------------------------------
    r + 1 > sqrt(2^64 + sqrt_input) * 2 - 2^33
    r + 1 + 2^33 > sqrt(2^64 + sqrt_input) * 2
    ((r+1)/2 + 2^32)^2 > 2^64 + sqrt_input

    Rewrite r as r = s * 2 + b (s = trunc(r/2), b is 0 or 1)

    ((s*2+b+1)/2 + 2^32)^2 > 2^64 + sqrt_input
    (s*2+b+1)^2/4 + 2*(s*2+b+1)/2*2^32 + 2^64 > 2^64 + sqrt_input
    (s*2+b+1)^2/4 + (s*2+b+1)*2^32 > sqrt_input
    (s*2+b+1)^2/4 + (r+1)*2^32 > sqrt_input
    (s*2+(b+1))^2/4 + r*2^32 + 2^32 > sqrt_input
    (s^2*4+2*s*2*(b+1)+(b+1)^2)/4 + r*2^32 + 2^32 > sqrt_input
    s^2+s*(b+1)+(b+1)^2/4 + r*2^32 + 2^32 > sqrt_input
    s*(s+b) + s + (b+1)^2/4 + r*2^32 + 2^32 > sqrt_input

    Let r2 = s*(s+b) + r*2^32

    r2 + s + (b+1)^2/4 + 2^32 > sqrt_input
    r2 + 2^32 + (b+1)^2/4 > sqrt_input - s

    If this inequality doesn't hold, then we must decrement r: IF "r2 + 2^32 + (b+1)^2/4 <= sqrt_input - s" THEN r = r - 1
    b can be 0 or 1
    If b is 0 then we need to compare "r2 + 2^32 + 1/4 <= sqrt_input - s" which is equal to "r2 + 2^32 < sqrt_input - s" because all numbers here are integers
    If b is 1 then (b+1)^2/4 = 1, so we need to compare "r2 + 2^32 + 1 <= sqrt_input - s" which is also equal to "r2 + 2^32 < sqrt_input - s"
    -----------------------------------------------------------------------------------
    Both cases can be merged to a single expression "r2 + 2^32 < sqrt_input - s"
    -----------------------------------------------------------------------------------
    There will be no overflow when calculating "r2 + 2^32":
    r2 + 2^32 = s*(s+b) + r*2^32 + 2^32 = s*(s+b) + (r+1)*2^32
    The largest value s, b and r can have is s = 1779033703, b = 1, r = 3558067407 when sqrt_input = 2^64 - 1
    r2 + b <= 1779033703*1779033704 + 3558067408*2^32 = 18446744072512414680 < 2^64

    There will be no integer overflow when calculating "sqrt_input - s", i.e. "sqrt_input >= s" at all times:
    s = trunc(r/2) = trunc(sqrt(2^64 + sqrt_input) - 2^32) < sqrt(2^64 + sqrt_input) - 2^32 + 1
    sqrt_input > sqrt(2^64 + sqrt_input) - 2^32 + 1
    sqrt_input + 2^32 - 1 > sqrt(2^64 + sqrt_input)
    (sqrt_input + 2^32 - 1)^2 > sqrt_input + 2^64
    sqrt_input^2 + 2*sqrt_input*(2^32 - 1) + (2^32-1)^2 > sqrt_input + 2^64
    sqrt_input^2 + sqrt_input*(2^33 - 2) + (2^32-1)^2 > sqrt_input + 2^64
    sqrt_input^2 + sqrt_input*(2^33 - 3) + (2^32-1)^2 > 2^64
    sqrt_input^2 + sqrt_input*(2^33 - 3) + 2^64-2^33+1 > 2^64
    sqrt_input^2 + sqrt_input*(2^33 - 3) - 2^33 + 1 > 0
    This inequality is true if sqrt_input > 1 and it's easy to check that s = 0 if sqrt_input is 0 or 1, so there will be no integer overflow
*/
#[inline]
pub(crate) fn variant2_integer_math_sqrt_fixup(sqrt_result: &mut u64, sqrt_input: u64) {
    let s = *sqrt_result >> 1;
    let b = *sqrt_result & 1;
    let r2 = s
        .wrapping_mul(s.wrapping_add(b))
        .wrapping_add(*sqrt_result << 32);
    *sqrt_result = sqrt_result
        .wrapping_add(if r2.wrapping_add(b) > sqrt_input {
            u64::MAX
        } else {
            0
        })
        .wrapping_add(if r2.wrapping_add(1u64 << 32) < sqrt_input - s {
            1
        } else {
            0
        });
}

// Reference implementation of the integer square root for Cryptonight variant 2
// Computes integer part of "sqrt(2^64 + n) * 2 - 2^33"
//
// In other words, given 64-bit unsigned integer n:
// 1) Write it as x = 1.NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN000... in binary (1 <= x < 2, all 64 bits of n are used)
// 2) Calculate sqrt(x) = 1.0RRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR... (1 <= sqrt(x) < sqrt(2), so it will always start with "1.0" in binary)
// 3) Take 32 bits that come after "1.0" and return them as a 32-bit unsigned integer, discard all remaining bits
//
// Some sample inputs and outputs:
//
// Input            | Output     | Exact value of "sqrt(2^64 + n) * 2 - 2^33"
// -----------------|------------|-------------------------------------------
// 0                | 0          | 0
// 2^32             | 0          | 0.99999999994179233909330885695244...
// 2^32 + 1         | 1          | 1.0000000001746229827200734316305...
// 2^50             | 262140     | 262140.00012206565608606978175873...
// 2^55 + 20963331  | 8384515    | 8384515.9999999997673963974959744...
// 2^55 + 20963332  | 8384516    | 8384516
// 2^62 + 26599786  | 1013904242 | 1013904242.9999999999479374853545...
// 2^62 + 26599787  | 1013904243 | 1013904243.0000000001561875439364...
// 2^64 - 1         | 3558067407 | 3558067407.9041987696409179931096...

// The reference implementation as it is now uses only unsigned int64 arithmetic, so it can't have undefined behavior
// It was tested once for all edge cases and confirmed correct

pub(crate) fn integer_square_root_v2(n: u64) -> u32 {
    let mut r = 1u64 << 63;
    let mut bit = 1u64 << 60;
    let mut n = n;

    while bit != 0 {
        let b = n < r.wrapping_add(bit);
        let n_next = n.wrapping_sub(r.wrapping_add(bit));
        let r_next = r.wrapping_add(bit << 1);
        n = if b { n } else { n_next };
        r = if b { r } else { r_next };
        r >>= 1;
        bit >>= 2;
    }

    let result = r.wrapping_mul(2) + if n > r { 1 } else { 0 };
    result as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_square_root_v2() {
        let test_cases = [
            (0, 0),
            (1 << 32, 0),
            ((1 << 32) + 1, 1),
            (1 << 50, 262140),
            ((1 << 55) + 20963331, 8384515),
            ((1 << 55) + 20963332, 8384516),
            ((1 << 62) + 26599786, 1013904242),
            ((1 << 62) + 26599787, 1013904243),
            (u64::MAX, 3558067407),
        ];

        for &(input, expected) in &test_cases {
            assert_eq!(integer_square_root_v2(input), expected, "input = {}", input);
        }
    }
}
