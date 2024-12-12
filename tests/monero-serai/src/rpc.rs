use std::{collections::BTreeSet, sync::atomic::Ordering};

use hex::serde::deserialize;
use monero_serai::block::Block;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::Deserialize;
use serde_json::json;

use crate::TESTED_BLOCK_COUNT;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BlockHeader {
    #[serde(deserialize_with = "deserialize")]
    pub hash: Vec<u8>,
    #[serde(deserialize_with = "deserialize")]
    pub miner_tx_hash: Vec<u8>,
    #[serde(deserialize_with = "deserialize")]
    pub prev_hash: Vec<u8>,

    pub block_weight: usize,
    pub height: usize,
    pub major_version: u8,
    pub minor_version: u8,
    pub nonce: u32,
    pub num_txes: usize,
    pub reward: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RpcClient {
    client: Client,
    rpc_node_url: String,
}

impl RpcClient {
    pub(crate) fn new(rpc_node_url: String) -> Self {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert("Content-Type", HeaderValue::from_static("application/json"));
            h
        };

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            rpc_node_url,
        }
    }

    pub(crate) async fn top_height(rpc_node_url: String) -> usize {
        #[derive(Debug, Clone, Deserialize)]
        struct JsonRpcResponse {
            result: GetLastBlockHeaderResponse,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub(crate) struct GetLastBlockHeaderResponse {
            pub block_header: BlockHeader,
        }

        let this = Self::new(rpc_node_url);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_last_block_header",
            "params": {}
        });

        this.client
            .get(this.rpc_node_url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap()
            .result
            .block_header
            .height
    }

    pub(crate) async fn get_block_test_batch(&self, heights: BTreeSet<usize>) {
        #[derive(Debug, Clone, Deserialize)]
        struct JsonRpcResponse {
            result: GetBlockResponse,
        }

        #[derive(Debug, Clone, Deserialize)]
        pub(crate) struct GetBlockResponse {
            #[serde(deserialize_with = "deserialize")]
            pub blob: Vec<u8>,
            pub block_header: BlockHeader,
        }

        let tasks = heights.into_iter().map(|height| {
            let request = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "get_block",
                "params": {"height": height}
            });

            let task =
                tokio::task::spawn(self.client.get(&self.rpc_node_url).json(&request).send());

            (height, task)
        });

        for (height, task) in tasks {
            let resp = task
                .await
                .unwrap()
                .unwrap()
                .json::<JsonRpcResponse>()
                .await
                .unwrap()
                .result;

            rayon::spawn(move || {
                let info = format!("\nheight: {height}\nresponse: {resp:#?}");

                // Test block deserialization.
                let block = match Block::read(&mut resp.blob.as_slice()) {
                    Ok(b) => b,
                    Err(e) => panic!("{e:?}\n{info}"),
                };

                // Test block properties.
                assert_eq!(resp.blob, block.serialize(), "{info}");

                assert!(
                    !block.miner_transaction.prefix().outputs.is_empty(),
                    "miner_tx has no outputs\n{info}"
                );

                let block_reward = block
                    .miner_transaction
                    .prefix()
                    .outputs
                    .iter()
                    .map(|o| o.amount.unwrap())
                    .sum::<u64>();
                assert_ne!(block_reward, 0, "block reward is 0\n{info}");

                // Test fields are correct.
                let BlockHeader {
                    block_weight,
                    hash,
                    height,
                    major_version,
                    minor_version,
                    miner_tx_hash,
                    nonce,
                    num_txes,
                    prev_hash,
                    reward,
                    timestamp,
                } = resp.block_header;

                assert_ne!(block_weight, 0, "{info}"); // TODO: test this
                assert_ne!(block.miner_transaction.weight(), 0, "{info}"); // TODO: test this
                assert_eq!(hash, block.hash(), "{info}");
                assert_eq!(height, block.number().unwrap(), "{info}");
                assert_eq!(major_version, block.header.hardfork_version, "{info}");
                assert_eq!(minor_version, block.header.hardfork_signal, "{info}");
                assert_eq!(miner_tx_hash, block.miner_transaction.hash(), "{info}");
                assert_eq!(nonce, block.header.nonce, "{info}");
                assert_eq!(num_txes, block.transactions.len(), "{info}");
                assert_eq!(prev_hash, block.header.previous, "{info}");
                assert_eq!(reward, block_reward, "{info}");
                assert_eq!(timestamp, block.header.timestamp, "{info}");

                TESTED_BLOCK_COUNT.fetch_add(1, Ordering::Release);
            });
        }
    }
}
