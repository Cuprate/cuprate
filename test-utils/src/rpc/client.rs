//! TODO

use std::sync::Arc;

use serde::Deserialize;
//---------------------------------------------------------------------------------------------------- Use
use serde_json::json;

use monero_serai::{
    block::Block,
    rpc::{HttpRpc, Rpc, RpcError},
    transaction::Transaction,
};

use cuprate_types::{ExtendedBlockHeader, TransactionVerificationData, VerifiedBlockInformation};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
///
/// TODO: Assumes non-restricted RPC.
pub struct HttpRpcClient {
    address: String,
    rpc: Rpc<HttpRpc>,
}

impl HttpRpcClient {
    /// TODO
    ///
    /// # Panics
    pub async fn new(address: Option<String>) -> Self {
        let address = address.unwrap_or_else(|| "http://127.0.0.1:18081".to_string());

        Self {
            rpc: HttpRpc::new(address.clone()).await.unwrap(),
            address,
        }
    }

    /// TODO
    const fn address(&self) -> &String {
        &self.address
    }

    /// TODO
    const fn rpc(&self) -> &Rpc<HttpRpc> {
        &self.rpc
    }

    /// TODO
    ///
    /// # Panics
    /// TODO
    pub async fn get_verified_block_information(&self, height: u64) -> VerifiedBlockInformation {
        #[derive(Debug, Deserialize)]
        struct Result {
            blob: String,
            block_header: BlockHeader,
            untrusted: bool,
        }

        #[derive(Debug, Deserialize)]
        struct BlockHeader {
            block_weight: usize,
            long_term_weight: usize,
            cumulative_difficulty: u128,
            hash: String,
            height: u64,
            pow_hash: String,
            reward: u64, // generated_coins
        }

        let result = self
            .rpc
            .json_rpc_call::<Result>(
                "get_block",
                Some(json!(
                    {
                        "height": height,
                        "fill_pow_hash": true
                    }
                )),
            )
            .await
            .unwrap();

        // Make sure this is a trusted, `pow_hash` only works there.
        assert!(
        	!result.untrusted,
        	"untrusted node detected, `pow_hash` will not show on these nodes - use a trusted node!"
        );

        let block = Block::read(&mut hex::decode(result.blob).unwrap().as_slice()).unwrap();

        let txs = self.rpc.get_transactions(&block.txs).await.unwrap();
        let txs = txs
            .into_iter()
            .enumerate()
            .map(|(i, tx)| {
                let tx_hash = tx.hash();
                assert_eq!(tx_hash, block.txs[i]);
                TransactionVerificationData {
                    tx_blob: tx.serialize(),
                    tx_weight: tx.weight(),
                    tx_hash,
                    fee: 0, // TODO: how to get this from RPC/calculate?
                    tx,
                }
            })
            .map(Arc::new)
            .collect();

        let block_header = result.block_header;
        let block_hash = <[u8; 32]>::try_from(hex::decode(block_header.hash).unwrap()).unwrap();
        let pow_hash = <[u8; 32]>::try_from(hex::decode(block_header.pow_hash).unwrap()).unwrap();

        // Assert the block hash matches.
        assert_eq!(block.hash(), block_hash);

        VerifiedBlockInformation {
            block,
            txs,
            block_hash,
            pow_hash,
            height: block_header.height,
            generated_coins: block_header.reward,
            weight: block_header.block_weight,
            long_term_weight: block_header.long_term_weight,
            cumulative_difficulty: block_header.cumulative_difficulty,
        }
    }
}
