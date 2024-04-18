//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;
use tokio::task::spawn_blocking;

use monero_serai::{
    block::Block,
    rpc::{HttpRpc, Rpc},
};

use cuprate_types::{TransactionVerificationData, VerifiedBlockInformation};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub struct HttpRpcClient {
    address: String,
    rpc: Rpc<HttpRpc>,
}

impl HttpRpcClient {
    /// Create an [`HttpRpcClient`].
    ///
    /// `address` should be an HTTP URL pointing to a `monerod`.
    ///
    /// If `None` is provided the default is used: `http://127.0.0.1:18081`.
    ///
    /// Note that for [`Self::get_verified_block_information`] to work, the `monerod`
    /// must be in unrestricted mode such that some fields (e.g. `pow_hash`) appear
    /// in the JSON response.
    ///
    /// # Panics
    /// This panics if the `address` is invalid or a connection could not be made.
    pub async fn new(address: Option<String>) -> Self {
        let address = address.unwrap_or_else(|| "http://127.0.0.1:18081".to_string());

        Self {
            rpc: HttpRpc::new(address.clone()).await.unwrap(),
            address,
        }
    }

    /// The address used for this [`HttpRpcClient`].
    #[allow(dead_code)]
    const fn address(&self) -> &String {
        &self.address
    }

    /// Access to the inner RPC client for other usage.
    #[allow(dead_code)]
    const fn rpc(&self) -> &Rpc<HttpRpc> {
        &self.rpc
    }

    /// Request data and map the response to a [`VerifiedBlockInformation`].
    ///
    /// # Panics
    /// This function will panic at any error point, e.g.,
    /// if the node cannot be connected to, if deserialization fails, etc.
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

        let (block_hash, block) = spawn_blocking(|| {
            let block = Block::read(&mut hex::decode(result.blob).unwrap().as_slice()).unwrap();
            (block.hash(), block)
        })
        .await
        .unwrap();

        let txs = self.rpc.get_transactions(&block.txs).await.unwrap();

        spawn_blocking(move || {
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
            let block_hash_2 =
                <[u8; 32]>::try_from(hex::decode(&block_header.hash).unwrap()).unwrap();
            let pow_hash =
                <[u8; 32]>::try_from(hex::decode(&block_header.pow_hash).unwrap()).unwrap();

            // Assert the block hash matches.
            assert_eq!(block_hash, block_hash_2);

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
        })
        .await
        .unwrap()
    }
}
