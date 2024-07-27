use bytemuck::{Pod, Zeroable};

use cuprate_types::CachedVerificationState;
use monero_serai::transaction::Timelock;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct RawCachedVerificationState {
    raw_valid_at_hash: [u8; 32],
    raw_hf: u8,
    raw_valid_past_timestamp: [u8; 8],
}

impl From<RawCachedVerificationState> for CachedVerificationState {
    fn from(value: RawCachedVerificationState) -> Self {
        if value.raw_valid_at_hash == [0; 32] {
            return CachedVerificationState::NotVerified;
        }

        let raw_valid_past_timestamp = u64::from_le_bytes(value.raw_valid_past_timestamp);

        if raw_valid_past_timestamp == 0 {
            return CachedVerificationState::ValidAtHashAndHF {
                block_hash: value.raw_valid_at_hash,
                hf: value.raw_hf,
            };
        }

        CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
            block_hash: value.raw_valid_at_hash,
            hf: value.raw_hf,
            time_lock: Timelock::Time(raw_valid_past_timestamp),
        }
    }
}

impl From<CachedVerificationState> for RawCachedVerificationState {
    fn from(value: CachedVerificationState) -> Self {
        match value {
            CachedVerificationState::NotVerified => Self {
                raw_valid_at_hash: [0; 32],
                raw_hf: 0,
                raw_valid_past_timestamp: [0; 8],
            },
            CachedVerificationState::ValidAtHashAndHF { block_hash, hf } => Self {
                raw_valid_at_hash: block_hash,
                raw_hf: hf,
                raw_valid_past_timestamp: [0; 8],
            },
            CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
                block_hash,
                hf,
                time_lock,
            } => {
                let Timelock::Time(time) = time_lock else {
                    panic!("ValidAtHashAndHFWithTimeBasedLock timelock was not time-based");
                };

                Self {
                    raw_valid_at_hash: block_hash,
                    raw_hf: hf,
                    raw_valid_past_timestamp: time.to_le_bytes(),
                }
            }
        }
    }
}
