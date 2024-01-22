//! This module contains an enum representing every Monero network: mainnet, testnet, stagenet and functionality
//! related to that.
//!
//! This feels out of place for the helper crate but this is needed through out Cuprate and felt too small to split
//! into it's own crate.
//!
//! `#[no_std]` compatible.

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
#[derive(Debug, Clone, Copy, Default)]
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
    pub fn network_id(&self) -> [u8; 16] {
        match self {
            Network::Mainnet => MAINNET_NETWORK_ID,
            Network::Testnet => TESTNET_NETWORK_ID,
            Network::Stagenet => STAGENET_NETWORK_ID,
        }
    }
}
