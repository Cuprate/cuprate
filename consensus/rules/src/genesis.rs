/// This module contains the code to generate Monero's genesis blocks.
///
/// ref: consensus-doc#Genesis
use monero_serai::{
    block::{Block, BlockHeader},
    transaction::Transaction,
};

use cuprate_helper::network::Network;

const fn genesis_nonce(network: &Network) -> u32 {
    match network {
        Network::Mainnet => 10000,
        Network::Testnet => 10001,
        Network::Stagenet => 10002,
    }
}

fn genesis_miner_tx(network: &Network) -> Transaction {
    Transaction::read(&mut hex::decode(match network {
        Network::Mainnet | Network::Testnet  => "013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1",
        Network::Stagenet => "013c01ff0001ffffffffffff0302df5d56da0c7d643ddd1ce61901c7bdc5fb1738bfe39fbe69c28a3a7032729c0f2101168d0c4ca86fb55a4cf6a36d31431be1c53a3bd7411bb24e8832410289fa6f3b"
    }).unwrap().as_slice()).unwrap()
}

/// Generates the Monero genesis block.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/genesis_block.html>
pub fn generate_genesis_block(network: &Network) -> Block {
    Block {
        header: BlockHeader {
            hardfork_version: 1,
            hardfork_signal: 0,
            timestamp: 0,
            previous: [0; 32],
            nonce: genesis_nonce(network),
        },
        miner_transaction: genesis_miner_tx(network),
        transactions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_genesis_blocks() {
        assert_eq!(
            &generate_genesis_block(&Network::Mainnet).hash(),
            hex::decode("418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3")
                .unwrap()
                .as_slice()
        );
        assert_eq!(
            &generate_genesis_block(&Network::Testnet).hash(),
            hex::decode("48ca7cd3c8de5b6a4d53d2861fbdaedca141553559f9be9520068053cda8430b")
                .unwrap()
                .as_slice()
        );
        assert_eq!(
            &generate_genesis_block(&Network::Stagenet).hash(),
            hex::decode("76ee3cc98646292206cd3e86f74d88b4dcc1d937088645e9b0cbca84b7ce74eb")
                .unwrap()
                .as_slice()
        );
    }
}
