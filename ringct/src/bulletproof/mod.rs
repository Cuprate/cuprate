//! Bulletproof
//!
//! Copied from https://github.com/dalek-cryptography/bulletproofs and
//! modified to mimic Monero's `bulletproof_PROVE` and `bulletproof_VERIFY` algorithms.

#![allow(non_snake_case)]

use core::iter;
use std::convert::TryFrom;

use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::{IsIdentity, VartimeMultiscalarMul};
use monero::util::ringct::CtKey;
use rand::{CryptoRng, RngCore};
use monero::Hash;

use inner_product_proof::InnerProductProof;
use crate::{EIGHT, INV_EIGHT, H};
use generators::{Hi, Gi};

mod generators;
mod inner_product_proof;
mod util;


const HASH_KEY_BULLETPROOF_EXPONENT: &[u8] = b"bulletproof";

const BULLETPROOF_MAX_OUTPUTS: usize = 16;
const MAX_M: usize = BULLETPROOF_MAX_OUTPUTS;
const N: usize = 64;

/// Verify that the `proof` is valid for the provided Pedersen commitments.
pub fn verify_bulletproof<T>(
    rng: &mut T,
    proof: monero::util::ringct::Bulletproof,
    commitments: Vec<CtKey>,
) -> Result<(), ProofError>
where
    T: RngCore + CryptoRng,
{
    let commitments: Result<Vec<EdwardsPoint>, ProofError> = commitments.iter().map(|key| 
        CompressedEdwardsY::from_slice(&key.mask.key).decompress().map_or(Err(ProofError::FormatError), |point| Ok(point * INV_EIGHT))
    ).collect();
    
    let proof = RangeProof::try_from(proof).map_err(|_| ProofError::FormatError)?;
    proof.verify_multiple_with_rng(&commitments?, rng)
}

/// Represents an error in proof creation, verification, or parsing.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProofError {
    /// This error occurs when a proof failed to verify.
    #[error("Proof verification failed.")]
    VerificationError,
    /// This error occurs when the proof encoding is malformed.
    #[error("Proof data could not be parsed.")]
    FormatError,
    /// This error occurs during proving if the number of blinding
    /// factors does not match the number of values.
    #[error("Wrong number of blinding factors supplied.")]
    WrongNumBlindingFactors,
    /// This error occurs when attempting to create a proof with
    /// bitsize other than \\(8\\), \\(16\\), \\(32\\), or \\(64\\).
    #[error("Invalid bitsize, must have n = 8,16,32,64.")]
    InvalidBitsize,
    /// This error occurs when attempting to create an aggregated
    /// proof with non-power-of-two aggregation size.
    #[error("Invalid aggregation size, m must be a power of 2.")]
    InvalidAggregation,
    /// This error occurs when there are insufficient generators for the proof.
    #[error("Invalid generators size, too few generators for proof")]
    InvalidGeneratorsLength,
}

/// The `RangeProof` struct represents a proof that one or more values
/// are in a range.
///
/// The `RangeProof` struct contains functions for creating and
/// verifying aggregated range proofs.  The single-value case is
/// implemented as a special case of aggregated range proofs.
///
/// The bitsize of the range, as well as the list of commitments to
/// the values, are not included in the proof, and must be known to
/// the verifier.
///
/// This implementation requires that both the bitsize `n` and the
/// aggregation size `m` be powers of two, so that `n = 8, 16, 32, 64`
/// and `m = 1, 2, 4, 8, 16, ...`.  Note that the aggregation size is
/// not given as an explicit parameter, but is determined by the
/// number of values or commitments passed to the prover or verifier.
#[derive(Clone, Debug)]
pub struct RangeProof {
    /// Commitment to the bits of the value
    A: EdwardsPoint,
    /// Commitment to the blinding factors
    S: EdwardsPoint,
    /// Commitment to the \\(t_1\\) coefficient of \\( t(x) \\)
    T_1: EdwardsPoint,
    /// Commitment to the \\(t_2\\) coefficient of \\( t(x) \\)
    T_2: EdwardsPoint,
    /// Evaluation of the polynomial \\(t(x)\\) at the challenge point \\(x\\)
    t_x: Scalar,
    /// Blinding factor for the synthetic commitment to \\(t(x)\\)
    t_x_blinding: Scalar,
    /// Blinding factor for the synthetic commitment to the inner-product arguments
    e_blinding: Scalar,
    /// Proof data for the inner-product argument.
    ipp_proof: InnerProductProof,
}

impl RangeProof {

    /// Verifies a rangeproof for a given value commitment \\(V\\).
    ///
    /// This is a convenience wrapper around `verify_multiple` for the
    /// `m=1` case.
    pub fn verify_single_with_rng<T: RngCore + CryptoRng>(
        &self,
        V: &EdwardsPoint,
        rng: &mut T,
    ) -> Result<(), ProofError> {
        self.verify_multiple_with_rng(&[*V], rng)
    }

    /// Verifies an aggregated rangeproof for the given value
    /// commitments.
    #[allow(clippy::many_single_char_names)]
    pub fn verify_multiple_with_rng<T: RngCore + CryptoRng>(
        &self,
        value_commitments: &[EdwardsPoint],
        rng: &mut T,
    ) -> Result<(), ProofError> {
        let m = value_commitments.len();

        let mut input = vec![];
        for commitment in value_commitments.iter() {
            input.extend_from_slice(commitment.compress().as_bytes());
        }

        let hash_commitments = Hash::hash_to_scalar(input).scalar;
        
        let mut input = hash_commitments.as_bytes().to_vec();
        input.extend_from_slice(self.A.compress().as_bytes());
        input.extend_from_slice(self.S.compress().as_bytes());


        let y = Hash::hash_to_scalar(input).scalar;

        if y == Scalar::zero() {
            return Err(ProofError::VerificationError);
        }

        let z = Hash::hash_to_scalar(y.as_bytes()).scalar;

        if z == Scalar::zero() {
            return Err(ProofError::VerificationError);
        }

        let zz = z * z;
        let minus_z = -z;

        let mut input = z.as_bytes().to_vec();
        input.extend_from_slice(z.as_bytes());
        input.extend_from_slice(self.T_1.compress().as_bytes());
        input.extend_from_slice(self.T_2.compress().as_bytes());

        let x = Hash::hash_to_scalar(input).scalar;

        if x == Scalar::zero() {
            return Err(ProofError::VerificationError);
        }

        let mut input = x.as_bytes().to_vec();
        input.extend_from_slice(x.as_bytes());
        input.extend_from_slice(self.t_x_blinding.as_bytes());
        input.extend_from_slice(self.e_blinding.as_bytes());
        input.extend_from_slice(self.t_x.as_bytes());

        let w = Hash::hash_to_scalar(input).scalar;

        if w == Scalar::zero() {
            return Err(ProofError::VerificationError);
        }

        // Challenge value for batching statements to be verified
        let c = Scalar::random(rng);

        let (x_sq, x_inv_sq, s) = self.ipp_proof.verification_scalars(N * m, w)?;
        let s_inv = s.iter().rev();

        let a = self.ipp_proof.a;
        let b = self.ipp_proof.b;

        // Construct concat_z_and_2, an iterator of the values of
        // z^0 * \vec(2)^n || z^1 * \vec(2)^n || ... || z^(m-1) * \vec(2)^n
        let powers_of_2: Vec<Scalar> = util::exp_iter(Scalar::from(2u64)).take(N).collect();
        let concat_z_and_2: Vec<Scalar> = util::exp_iter(z)
            .take(m)
            .flat_map(|exp_z| powers_of_2.iter().map(move |exp_2| exp_2 * exp_z))
            .collect();

        let g = s.iter().map(|s_i| minus_z - a * s_i);
        let h = s_inv
            .zip(util::exp_iter(y.invert()))
            .zip(concat_z_and_2.iter())
            .map(|((s_i_inv, exp_y_inv), z_and_2)| z + exp_y_inv * (zz * z_and_2 - b * s_i_inv));

        let value_commitment_scalars = util::exp_iter(z).take(m).map(|z_exp| c * zz * z_exp);
        let basepoint_scalar = w * (self.t_x - a * b) + c * (delta(N, m, &y, &z) - self.t_x);

        let mega_check = EdwardsPoint::optional_multiscalar_mul(
            iter::once(Scalar::one())
                .chain(iter::once(x))
                .chain(iter::once(c * x))
                .chain(iter::once(c * x * x))
                .chain(x_sq.iter().cloned())
                .chain(x_inv_sq.iter().cloned())
                .chain(iter::once(-self.e_blinding - c * self.t_x_blinding))
                .chain(iter::once(basepoint_scalar))
                .chain(g)
                .chain(h)
                .chain(value_commitment_scalars),
            iter::once(EIGHT * self.A)
                .chain(iter::once(EIGHT * self.S))
                .chain(iter::once(EIGHT * self.T_1))
                .chain(iter::once(EIGHT * self.T_2))
                .chain(self.ipp_proof.L_vec.iter().map(|L| EIGHT * L))
                .chain(self.ipp_proof.R_vec.iter().map(|R| EIGHT * R))
                .chain(iter::once(ED25519_BASEPOINT_POINT))
                .chain(iter::once(*H))
                .chain(Gi[0..m*N].to_owned())
                .chain(Hi[0..m*N].to_owned())
                .chain(value_commitments.iter().map(|V| EIGHT * V))
                .map(Some),
        )
        .ok_or(ProofError::VerificationError)?;

        if mega_check.is_identity() {
            Ok(())
        } else {
            Err(ProofError::VerificationError)
        }
    }
}

/// Compute
/// \\[
/// \delta(y,z) = (z - z^{2}) \langle \mathbf{1}, {\mathbf{y}}^{n \cdot m} \rangle - \sum_{j=0}^{m-1} z^{j+3} \cdot \langle \mathbf{1}, {\mathbf{2}}^{n \cdot m} \rangle
/// \\]
fn delta(n: usize, m: usize, y: &Scalar, z: &Scalar) -> Scalar {
    let sum_y = util::sum_of_powers(y, n * m);
    let sum_2 = util::sum_of_powers(&Scalar::from(2u64), n);
    let sum_z = util::sum_of_powers(z, m);

    (z - z * z) * sum_y - z * z * z * sum_2 * sum_z
}


#[derive(Debug)]
pub enum ConversionError {
    InvalidPoint,
    NonCanonicalScalar,
}

impl TryFrom<monero::util::ringct::Bulletproof> for RangeProof {
    type Error = ConversionError;

    fn try_from(from: monero::util::ringct::Bulletproof) -> Result<Self, ConversionError> {
        Ok(Self {
            A: CompressedEdwardsY::from_slice(&from.A.key)
                .decompress()
                .ok_or(ConversionError::InvalidPoint)?,
            S: CompressedEdwardsY::from_slice(&from.S.key)
                .decompress()
                .ok_or(ConversionError::InvalidPoint)?,
            T_1: CompressedEdwardsY::from_slice(&from.T1.key)
                .decompress()
                .ok_or(ConversionError::InvalidPoint)?,
            T_2: CompressedEdwardsY::from_slice(&from.T2.key)
                .decompress()
                .ok_or(ConversionError::InvalidPoint)?,
            t_x: Scalar::from_canonical_bytes(from.t.key)
                .ok_or(ConversionError::NonCanonicalScalar)?,
            t_x_blinding: Scalar::from_canonical_bytes(from.taux.key)
                .ok_or(ConversionError::NonCanonicalScalar)?,
            e_blinding: Scalar::from_canonical_bytes(from.mu.key)
                .ok_or(ConversionError::NonCanonicalScalar)?,
            ipp_proof: InnerProductProof {
                L_vec: from
                    .L
                    .iter()
                    .map(|L| {
                        CompressedEdwardsY::from_slice(&L.key)
                            .decompress()
                            .ok_or(ConversionError::InvalidPoint)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                R_vec: from
                    .R
                    .iter()
                    .map(|R| {
                        CompressedEdwardsY::from_slice(&R.key)
                            .decompress()
                            .ok_or(ConversionError::InvalidPoint)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                a: Scalar::from_canonical_bytes(from.a.key)
                    .ok_or(ConversionError::NonCanonicalScalar)?,
                b: Scalar::from_canonical_bytes(from.b.key)
                    .ok_or(ConversionError::NonCanonicalScalar)?,
            },
        })
    }
}

#[test]
fn test() {
        let hex = hex::decode("02000102000bb18c8704ef9b1eed37b20baa04c12892015bdb01d00e01578c036a052afe921d3860d7452b454783a5f2facd519260623a6c8efd257e800200022ec69e512719b6059212839883b8631ff4b6206d04b6a70ed2f85c456167ad8700025f0933bb0e641af0554d5287bc63d6636ebc4871d04c69f3491f7c27cd73ea502101963209775677af7861b43d864b5feedd6b7678162b5601aa704a5bca12e89a1503b0f79a12abf75abaa1aad48c59916978d67924e8d0c2873d0fe7a49cdafb64021d7120063d28af02568b6be3574f4cf438bba51a8be775997d7f5b732371f98f1bd74704eae6b019853fe3cece79a2c27c1ebd795b330c50a84515b68ee4c79094d5d004d65d7c453a873ca12b50c0714da4703037a3547575002eb1645b56cff539be0a80e8359f32f51394219f814bc4476fe8bde75d05a28acd5a13ed8945335e508d2cf69e164ddd4cd02a9d01655a2c0e2b265b8ef58ca060dd617c2a6a30fe28110100000094a29f80580e57f2abfd26786f6cb3cbfe86c0a339b6f8b04649075207a31229bb7b91f6661ba58d532a8c8fe7dfce004ec729cb8af8205c14f49755ad2fb05c9deeec330068163d76e9d71f0590d0d4c69c4a6360bbf3a869ac81f383ebd0ce55700ff0453681c3984055b6b9809319b97ac9ee692c2ddb595e9f0623d818004870aa4ecb31c0605b533c8501604464275226b145acbeccff50866744d65d0da04cfe116f206adb2a8823798f20537126bb2fb7abbdd1c6b7a37b3e98012c0007d388bf8830965158b98f93a3f2cfbf2adbfe2d35b51552224ccafdb9b70583550def81d198f99dd81762a9b032813b123aeba129f8c7c137efb708335a61d2b32ca8576d1a77aa71a6a0565068e86f78b56d91deb86fabf803e9390e81461b1f5ff32e3a614b9b639fdc8c39c5604455e991ef4ac2b5626c6e99f6d7813ccfd2eaf0bd51845f43c2c681aba12e18ac9d367f593905b4cb8546aaed4cffd29a3f974b44718faa2fd23010eb6a65f59d33fd88d73b92e05ac26657855003e60f998c4fe38fc4a27b0d0690e607569d6e6e26cf0f1645f6bbec09c5e30c259ca78d07d020520baa0498dc65125c0063142a3e57704b3f1d6972d6c0e0da569987944d537b7ec7a271c02ea22a362a6b7a229c77c4836ddd62fa8e7151379b6281bf37a976842ec7abe7c4d7245ec891cb95ffd1ad00932186062229d8a784017e897f55287c5cd914e94765e2d6c6afd709dd309f83e66e15dca4fb6139584f87affe92836f7560b1428dd745496d769b2ff6333068d69f0b7a97836c271d08c7a394f1ef2071b492b3ea1583a4e6f056b2ea9ffbcf9085be466f9a0dc1f85cdb260a544f183c38b6a748c23cac81808c38e826be35fde200a95f5d742f5655fa57db43c13fd0578bd571e486a07858f9846479cad5b2758abcf1a4ef0ed59d96ab0ef23676bab9935a60b648378a1d343c9069630456e0f90906c5dd4d134f1bce00c6086a0238ebe8fbfe2f03d1eef8e01d1b09066430fe6f0963df625bb57e5a088a2bd71156030f9b2c997fa683a0cc1bca3dda67f8694497091e898192977c0540da0ec9c6a040671f08c604c03e201c3e80fbf69555e3ad7e317af988c56b08aa503dd8adcbec6170db22ceb71c7c9194b507f93793bd808d76f16d1ecc480e0e56de8fca357f8e35174453803a6b8012ca2a459d4a9fd88b33a6ccaea11a0eb0fa40851c9b123b627682fe627c3f52251a25bdbbd6e2a286500dde223d8006c5ac6d0ce9e8b70d3b0208b103f682d3f9a589210e64e7507cd37301c166fb0f664e1fa455f3cf99735e21321fece75ac06051a34a3b2c8e12b9d4629d102500d5d547072a5890c8641ad37b20f8710a6688cff28d11007bafa3a9a35c63b30f92d892eb5a501c7d1d860a770ca47dcf8e8d9fd5b8072c4eaab28791f5c6cb0de7a2a7381ecb8b697a2a38c30270bbbaab7366eb70ce35c749798b4e49e90f05af2f7f832b2f646e9b9c9140846a15cd502ce01100e60fdd513008752704d303f5aa3867ea5e144c33556d0c59eb7bbbe36556b82b6b38a0a87f3b2c3a91920d9c3df9ce7ba2d27b7bc0c9ac89dbc99ef9ef2c86cae4df4803887d00c3a5040451b1f8b13c75e94fe7e4ade68df662afea93a83d4c79cebfe4a21642825bb1080be15496065a810069b5e14339a9fabc87c4fec6e4dfbf6a400b421743a1500d8d4f4317d6b08af1454692a513315ce5e7b46530ce45ed36f9d9a4bd6dda9e0c9486b8486e2c079c770550e88f153fb25819f7bd14d9768051b1e28123a8450ee915d487b75465e80de9fbc21c87093baec245dca49fd537aaa3404d8c80b40d3d4277e31cb23a3c234e905a16d2460a1d72b0417cb7170d73ea5c66335f840fe6856d124d0df9a57151c2fb6f85c43907583add541fbcfe2e7279e4664ba201f41418d45aa2bec87f59faf9a890fc994b9c15302fd47eaf4b8e326d7480d103cc48ea5ade3b8838731b04f837ff7057218ca019bce207de7b95614dd7020f03e673b2b2ff9fe237aeb49fe6a6e33038b4d43a8f10d6601613e3978ebf66df0ecf4bf4ca6dc846a0b7ecfd1279c827304d7e0be315446930a4065422394875e8").unwrap();
        let tx = monero::consensus::deserialize::<monero::Transaction>(&hex[..]).unwrap();
        let v =  tx.rct_signatures.sig.unwrap().out_pk;
        let rs = &tx.rct_signatures.p.as_ref().unwrap().bulletproofs[0];

        let mut rng = rand::thread_rng();
        let now = std::time::Instant::now();
        for _ in 0..100 {
            verify_bulletproof(&mut rng, rs.clone(), v.clone()).unwrap();

        }
        println!("{}", now.elapsed().as_millis() /100);


}