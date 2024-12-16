use std::fmt::Display;

use hex_literal::hex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CryptoNightHash {
    V0,
    V1,
    V2,
    R,
}

impl CryptoNightHash {
    /// The last height this hash function is used for proof-of-work.
    pub(crate) const fn from_height(height: u64) -> Self {
        if height < 1546000 {
            Self::V0
        } else if height < 1685555 {
            Self::V1
        } else if height < 1788000 {
            Self::V2
        } else if height < 1978433 {
            Self::R
        } else {
            panic!("height is large than 1978433");
        }
    }

    pub(crate) fn hash(data: &[u8], height: u64) -> (&'static str, [u8; 32]) {
        let this = Self::from_height(height);

        let hash = match Self::from_height(height) {
            Self::V0 => {
                if height == 202612 {
                    hex!("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000")
                } else {
                    cuprate_cryptonight::cryptonight_hash_v0(data)
                }
            }
            Self::V1 => cuprate_cryptonight::cryptonight_hash_v1(data).unwrap(),
            Self::V2 => cuprate_cryptonight::cryptonight_hash_v2(data),
            Self::R => cuprate_cryptonight::cryptonight_hash_r(data, height),
        };

        (this.as_str(), hash)
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::V0 => "cryptonight_v0",
            Self::V1 => "cryptonight_v1",
            Self::V2 => "cryptonight_v2",
            Self::R => "cryptonight_r",
        }
    }
}

impl Display for CryptoNightHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).as_str())
    }
}
