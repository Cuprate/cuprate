//! This module contains an enum representing every Monero network: mainnet, testnet, stagenet and functionality
//! related to that.
//!
//! This feels out of place for the helper crate but this is needed through out Cuprate and felt too small to split
//! into it's own crate.
//!
//! `#[no_std]` compatible.
// TODO: move to types crate.

use core::{
    fmt::{Display, Formatter},
    str::FromStr,
};

const MAINNET_NETWORK_ID: [u8; 16] = [
    0x12, 0x30, 0xF1, 0x71, 0x61, 0x04, 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x10,
];
const TESTNET_NETWORK_ID: [u8; 16] = [
    0x12, 0x30, 0xF1, 0x71, 0x61, 0x04, 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x11,
];
const STAGENET_NETWORK_ID: [u8; 16] = [
    0x12, 0x30, 0xF1, 0x71, 0x61, 0x04, 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x12,
];

/// An enum representing every Monero network.
#[derive(Debug, Clone, Copy, Default, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Network {
    /// Mainnet
    #[default]
    Mainnet,
    /// Testnet
    Testnet,
    /// Stagenet
    Stagenet,
    /// Regtest (fakechain).
    FakeChain,
}

impl Network {
    /// Returns the network ID for the current network.
    pub const fn network_id(&self) -> [u8; 16] {
        match self {
            Self::Mainnet | Self::FakeChain => MAINNET_NETWORK_ID,
            Self::Testnet => TESTNET_NETWORK_ID,
            Self::Stagenet => STAGENET_NETWORK_ID,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseNetworkError;

impl core::error::Error for ParseNetworkError {}

impl Display for ParseNetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(
            r#"invalid network, expected one of "Mainnet", "Testnet", "Stagenet", "FakeChain" (case-insensitive)"#,
        )
    }
}

impl FromStr for Network {
    type Err = ParseNetworkError;

    /// Parses a [`Network`] from a string, ignoring ASCII case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s.eq_ignore_ascii_case("mainnet") => Ok(Self::Mainnet),
            s if s.eq_ignore_ascii_case("testnet") => Ok(Self::Testnet),
            s if s.eq_ignore_ascii_case("stagenet") => Ok(Self::Stagenet),
            s if s.eq_ignore_ascii_case("fakechain") => Ok(Self::FakeChain),
            _ => Err(ParseNetworkError),
        }
    }
}
impl Display for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Stagenet => "stagenet",
            Self::FakeChain => "fakechain",
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_str_ignores_ascii_case() {
        for (input, expected) in [
            ("mainnet", Network::Mainnet),
            ("Mainnet", Network::Mainnet),
            ("MAINNET", Network::Mainnet),
            ("testnet", Network::Testnet),
            ("Testnet", Network::Testnet),
            ("TESTNET", Network::Testnet),
            ("stagenet", Network::Stagenet),
            ("Stagenet", Network::Stagenet),
            ("STAGENET", Network::Stagenet),
            ("fakechain", Network::FakeChain),
            ("FakeChain", Network::FakeChain),
            ("FAKECHAIN", Network::FakeChain),
        ] {
            assert_eq!(input.parse(), Ok(expected));
        }
    }

    #[test]
    fn from_str_rejects_invalid_networks() {
        for input in ["", "mainnet2", " mainnet", "main net"] {
            assert_eq!(input.parse::<Network>(), Err(ParseNetworkError));
        }
    }
}
