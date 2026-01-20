use crate::{POWER_PERSONALIZATION_STRING, PowerChallenge};

const SIZE: usize = POWER_PERSONALIZATION_STRING.len()
    + size_of::<[u8; 32]>()
    + size_of::<[u8; 32]>()
    + size_of::<u32>();

const _: () = assert!(SIZE == 80);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
/// A [`PowerChallenge`] for the RPC interface.
///
/// This creates well-formed challenges for RPC.
pub struct PowerChallengeRpc([u8; SIZE]);

impl AsRef<[u8]> for PowerChallengeRpc {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; SIZE]> for PowerChallengeRpc {
    fn from(t: [u8; SIZE]) -> Self {
        Self(t)
    }
}

impl From<PowerChallengeRpc> for [u8; SIZE] {
    fn from(t: PowerChallengeRpc) -> Self {
        t.0
    }
}

impl PowerChallenge for PowerChallengeRpc {
    /// `(tx_prefix_hash || recent_block_hash || nonce)`
    type ChallengeInput = ([u8; 32], [u8; 32], u32);

    const SIZE: usize = SIZE;

    fn new(challenge: &[u8]) -> Option<Self> {
        match challenge.try_into() {
            Ok(t) => Some(Self(t)),
            Err(_) => None,
        }
    }

    fn new_from_input(input: Self::ChallengeInput) -> Self {
        let tx_prefix_hash = input.0;
        let recent_block_hash = input.1;
        let nonce = input.2;

        let mut this = [0; SIZE];
        this[..12].copy_from_slice(POWER_PERSONALIZATION_STRING.as_bytes());
        this[12..44].copy_from_slice(&tx_prefix_hash);
        this[44..76].copy_from_slice(&recent_block_hash);
        this[76..].copy_from_slice(&u32::to_le_bytes(nonce));

        Self(this)
    }

    fn update_nonce(&mut self, nonce: u32) {
        self.0[76..].copy_from_slice(&u32::to_le_bytes(nonce));
    }

    fn nonce(&self) -> u32 {
        u32::from_le_bytes(self.0[76..80].try_into().unwrap())
    }
}
