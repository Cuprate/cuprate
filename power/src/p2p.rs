use crate::PowerChallenge;

const SIZE: usize = size_of::<u64>() + size_of::<u64>() + size_of::<u32>();

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
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
    /// `(power_challenge_nonce || nonce)`
    type ChallengeInput = (u128, u32);

    const SIZE: usize = SIZE;

    fn new(challenge: &[u8]) -> Option<Self> {
        match challenge.try_into() {
            Ok(t) => Some(Self(t)),
            Err(_) => None,
        }
    }

    fn new_from_input(input: Self::ChallengeInput) -> Self {
        let power_challenge_nonce = input.0;
        let nonce = input.1;

        let mut this = [0; SIZE];
        this[..16].copy_from_slice(&u128::to_le_bytes(power_challenge_nonce));
        this[16..].copy_from_slice(&u32::to_le_bytes(nonce));

        Self(this)
    }

    fn update_nonce(&mut self, nonce: u32) {
        self.0[16..].copy_from_slice(&u32::to_le_bytes(nonce));
    }
}
