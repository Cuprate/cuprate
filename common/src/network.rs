

const MAINNET_NETWORK_ID: [u8; 16] = [
    0x12, 0x30, 0xF1, 0x71, 0x61, 0x04, 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x10,
];
const TESTNET_NETWORK_ID: [u8; 16] = [
    0x12, 0x30, 0xF1, 0x71, 0x61, 0x04, 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x11,
];
const STAGENET_NETWORK_ID: [u8; 16] = [
    0x12, 0x30, 0xF1, 0x71, 0x61, 0x04, 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x12,
];

pub enum Network {
    MainNet,
    TestNet,
    StageNet,
}

impl Network {
    pub fn network_id(&self) -> [u8; 16] {
        match self {
            Network::MainNet => MAINNET_NETWORK_ID,
            Network::TestNet => TESTNET_NETWORK_ID,
            Network::StageNet => STAGENET_NETWORK_ID,
        }
    }
}
