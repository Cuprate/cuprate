use crate::{POWER_PERSONALIZATION_STRING, PowerChallenge};

const SIZE: usize = POWER_PERSONALIZATION_STRING.len()
    + size_of::<u64>()
    + size_of::<u64>()
    + size_of::<u32>()
    + size_of::<u32>();

const _: () = assert!(SIZE == 36);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
/// A [`PowerChallenge`] for the P2P interface.
///
/// This creates well-formed challenges for P2P.
pub struct PowerChallengeP2p([u8; SIZE]);

impl AsRef<[u8]> for PowerChallengeP2p {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; SIZE]> for PowerChallengeP2p {
    fn from(t: [u8; SIZE]) -> Self {
        Self(t)
    }
}

impl From<PowerChallengeP2p> for [u8; SIZE] {
    fn from(t: PowerChallengeP2p) -> Self {
        t.0
    }
}

impl PowerChallenge for PowerChallengeP2p {
    /// `(seed || seed_top64 || difficulty || nonce)`
    type ChallengeInput = (u64, u64, u32, u32);

    const SIZE: usize = SIZE;

    fn new(challenge: &[u8]) -> Option<Self> {
        match challenge.try_into() {
            Ok(t) => Some(Self(t)),
            Err(_) => None,
        }
    }

    fn new_from_input(input: Self::ChallengeInput) -> Self {
        let seed = {
            let (low, high) = (input.0, input.1);
            (u128::from(high) << 64) | u128::from(low)
        };
        let difficulty = input.2;
        let nonce = input.3;

        let mut this = [0; SIZE];

        this[..12].copy_from_slice(POWER_PERSONALIZATION_STRING.as_bytes());
        this[12..28].copy_from_slice(&u128::to_le_bytes(seed));
        this[28..32].copy_from_slice(&u32::to_le_bytes(difficulty));
        this[32..36].copy_from_slice(&u32::to_le_bytes(nonce));

        Self(this)
    }

    fn update_nonce(&mut self, nonce: u32) {
        self.0[32..].copy_from_slice(&u32::to_le_bytes(nonce));
    }

    fn nonce(&self) -> u32 {
        u32::from_le_bytes(self.0[32..36].try_into().unwrap())
    }
}
