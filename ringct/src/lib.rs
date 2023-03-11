pub mod borromean;
pub mod bulletproof;

use curve25519_dalek::{edwards::EdwardsPoint, scalar::Scalar};
use monero::util::key::H as CompressedH;
use lazy_static::lazy_static;

/// Defines the constant 1/8.
///
/// Monero likes to multiply things by 8 as part of CLSAG, Bulletproof and key derivation.
/// As such, we also sometimes need to multiply by its inverse to undo this operation.
///
/// We define the constant here once instead of littering it across the codebase.
const INV_EIGHT: Scalar = Scalar::from_bits([
    121, 47, 220, 226, 41, 229, 6, 97, 208, 218, 28, 125, 179, 157, 211, 7, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 6,
]);

/// Defines the constant 8.
///
/// Monero likes to multiply things by 8 as part of CLSAG, Bulletproof and key derivation.
/// We define the constant here once instead of littering it across the codebase.
const EIGHT: Scalar = Scalar::from_bits([
    8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
]);


lazy_static! {
    static ref H: EdwardsPoint = CompressedH.point.decompress().unwrap();
    static ref H2: [EdwardsPoint; 64] = generate_H2();
}

#[allow(non_snake_case)]
fn generate_H2() -> [EdwardsPoint; 64] {
    let mut temp = Vec::with_capacity(64);
    for i in 0..64 {
        temp.push(Scalar::from(2_u128.pow(i as u32)) * *H)
    }
    temp.try_into().unwrap()
}
