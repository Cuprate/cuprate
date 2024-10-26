//! This module contains an enum representing every Monero network: mainnet, testnet, stagenet and functionality
//! related to that.
//!
//! This feels out of place for the helper crate but this is needed through out Cuprate and felt too small to split
//! into it's own crate.
//!
//! `#[no_std]` compatible.
// TODO: move to types crate.
use std::{
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
}

impl Network {
    /// Returns the network ID for the current network.
    pub const fn network_id(&self) -> [u8; 16] {
        match self {
            Self::Mainnet => MAINNET_NETWORK_ID,
            Self::Testnet => TESTNET_NETWORK_ID,
            Self::Stagenet => STAGENET_NETWORK_ID,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseNetworkError;

impl FromStr for Network {
    type Err = ParseNetworkError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            "stagenet" => Ok(Self::Stagenet),
            _ => Err(ParseNetworkError),
        }
    }
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Stagenet => "stagenet",
        })
    }
}
