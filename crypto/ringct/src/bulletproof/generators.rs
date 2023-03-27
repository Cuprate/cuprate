//! The `generators` module contains API for producing a set of
//! generators for a rangeproof.

#![allow(non_upper_case_globals)]


use curve25519_dalek::edwards:: EdwardsPoint;
use hash_edwards_to_edwards::hash_to_point;
use monero::{Hash, VarInt};
use lazy_static::lazy_static;

use super::{HASH_KEY_BULLETPROOF_EXPONENT, MAX_M, N};
use crate::H;

lazy_static! {
    pub static ref Hi: [EdwardsPoint; MAX_M *N] = generate_Hi();
    pub static ref Gi: [EdwardsPoint; MAX_M * N] = generate_Gi();
}


fn generate_Hi() -> [EdwardsPoint; MAX_M * N] {
    let mut hi = Vec::with_capacity(N*MAX_M);
    for i in 0..N*MAX_M  {
        hi.push(get_exponent(*H, i * 2));
    }
    hi.try_into().unwrap()
}

fn generate_Gi() -> [EdwardsPoint; MAX_M * N] {
    let mut gi = Vec::with_capacity(N*MAX_M);
    for i in 0..N*MAX_M  {
        gi.push(get_exponent(*H, (i * 2)+1));
    }
    gi.try_into().unwrap()
}

/// https://github.com/monero-project/monero/blob/release-v0.17/src/ringct/bulletproofs.cc#L101-L111
fn get_exponent(base: EdwardsPoint, index: usize) -> EdwardsPoint {
    let mut input = base.compress().as_bytes().to_vec();
    input.extend_from_slice(HASH_KEY_BULLETPROOF_EXPONENT);
    input.extend_from_slice(&monero::consensus::serialize(&VarInt(index.try_into().unwrap())));

    let output = Hash::new(input);
    hash_to_point(output.as_fixed_bytes())
}


