use std::time::Duration;

use crossbeam::channel::Sender;
use hex::serde::deserialize;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{VerifyData, RANDOMX_START_HEIGHT};

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    result: GetBlockResponse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetBlockResponse {
    #[serde(deserialize_with = "deserialize")]
    pub blob: Vec<u8>,
    pub block_header: BlockHeader,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockHeader {
    #[serde(deserialize_with = "deserialize")]
    pub pow_hash: Vec<u8>,
    #[serde(deserialize_with = "deserialize")]
    pub hash: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct RpcClient {
    client: Client,
    json_rpc_url: String,
    pub top_height: u64,
}

impl RpcClient {
    pub(crate) async fn new(rpc_url: String) -> Self {
        let headers = {
            let mut h = HeaderMap::new();
            h.insert("Content-Type", HeaderValue::from_static("application/json"));
            h
        };

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_last_block_header",
            "params": {}
        });

        let json_rpc_url = format!("{rpc_url}/json_rpc");

        let top_height = client
            .get(&json_rpc_url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()
            .get("result")
            .unwrap()
            .get("block_header")
            .unwrap()
            .get("height")
            .unwrap()
            .as_u64()
            .unwrap();

        assert!(top_height > 3301441, "node is behind");

        Self {
            client,
            json_rpc_url,
            top_height,
        }
    }

    async fn get_block(&self, height: u64) -> GetBlockResponse {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "get_block",
            "params": {"height": height, "fill_pow_hash": true}
        });

        self.client
            .get(&self.json_rpc_url)
            .json(&request)
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap()
            .result
    }

    pub(crate) async fn test(self, top_height: u64, tx: Sender<VerifyData>) {
        use futures::StreamExt;

        let iter = (0..top_height).map(|height| {
            let this = &self;
            let tx = tx.clone();

            async move {
                let get_block_response = this.get_block(height).await;

                let (seed_height, seed_hash) = if height < RANDOMX_START_HEIGHT {
                    (0, [0; 32])
                } else {
                    let seed_height = cuprate_consensus_rules::blocks::randomx_seed_height(
                        height.try_into().unwrap(),
                    )
                    .try_into()
                    .unwrap();

                    let seed_hash = this
                        .get_block(seed_height)
                        .await
                        .block_header
                        .hash
                        .try_into()
                        .unwrap();

                    (seed_height, seed_hash)
                };

                let data = VerifyData {
                    get_block_response,
                    height,
                    seed_height,
                    seed_hash,
                };

                tx.send(data).unwrap();
            }
        });

        futures::stream::iter(iter)
            .buffer_unordered(4) // This can't be too high or else we get bottlenecked by `monerod`
            .for_each(|()| async {})
            .await;
    }
}
