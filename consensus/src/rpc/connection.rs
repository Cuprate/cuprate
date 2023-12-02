use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
    task::{Context, Poll},
};

use curve25519_dalek::edwards::CompressedEdwardsY;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use monero_serai::{
    block::Block,
    rpc::{HttpRpc, Rpc},
    transaction::Transaction,
};
use monero_wire::common::{BlockCompleteEntry, TransactionBlobs};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    task::JoinHandle,
    time::{timeout, Duration},
    sync::RwLock
};
use tower::Service;
use tracing::{instrument, Instrument};

use cuprate_common::{tower_utils::InfallibleOneshotReceiver, BlockID};

use super::ScanningCache;
use crate::{
    helper::rayon_spawn_async, DatabaseRequest, DatabaseResponse, ExtendedBlockHeader, HardFork,
    OutputOnChain,
};
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
const OUTPUTS_TIMEOUT: Duration = Duration::from_secs(20);

pub struct RpcConnectionSvc {
    pub(crate) address: String,

    pub(crate) rpc_task_handle: JoinHandle<()>,
    pub(crate) rpc_task_chan: mpsc::Sender<RpcReq>,
}

impl Service<DatabaseRequest> for RpcConnectionSvc {
    type Response = DatabaseResponse;
    type Error = tower::BoxError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.rpc_task_handle.is_finished() {
            return Poll::Ready(Err("RPC task has exited!".into()));
        }
        self.rpc_task_chan.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: DatabaseRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let req = RpcReq {
            req,
            res_chan: tx,
            span: tracing::info_span!(parent: &tracing::Span::current(), "rpc", addr = &self.address),
        };

        self.rpc_task_chan
            .try_send(req)
            .expect("poll_ready should be called first!");

        rx.into()
    }
}

pub(crate) struct RpcReq {
    req: DatabaseRequest,
    res_chan: oneshot::Sender<Result<DatabaseResponse, tower::BoxError>>,
    span: tracing::Span,
}

pub struct RpcConnection {
    pub(crate) address: String,

    pub(crate) con: Rpc<HttpRpc>,
    pub(crate) cache: Arc<RwLock<ScanningCache>>,

    pub(crate) req_chan: mpsc::Receiver<RpcReq>,
}

impl RpcConnection {
    async fn get_block_hash(&self, height: u64) -> Result<[u8; 32], tower::BoxError> {
        self.con
            .get_block_hash(height.try_into().unwrap())
            .await
            .map_err(Into::into)
    }

    async fn get_extended_block_header(
        &self,
        id: BlockID,
    ) -> Result<ExtendedBlockHeader, tower::BoxError> {
        tracing::info!("Retrieving block info with id: {}", id);

        #[derive(Deserialize, Debug)]
        struct Response {
            block_header: BlockInfo,
        }

        let info = match id {
            BlockID::Height(height) => {
                let res = self
                    .con
                    .json_rpc_call::<Response>(
                        "get_block_header_by_height",
                        Some(json!({"height": height})),
                    )
                    .await?;
                res.block_header
            }
            BlockID::Hash(hash) => {
                let res = self
                    .con
                    .json_rpc_call::<Response>(
                        "get_block_header_by_hash",
                        Some(json!({"hash": hash})),
                    )
                    .await?;
                res.block_header
            }
        };

        Ok(ExtendedBlockHeader {
            version: HardFork::from_version(&info.major_version)
                .expect("previously checked block has incorrect version"),
            vote: HardFork::from_vote(&info.minor_version),
            timestamp: info.timestamp,
            cumulative_difficulty: u128_from_low_high(
                info.cumulative_difficulty,
                info.cumulative_difficulty_top64,
            ),
            block_weight: info.block_weight,
            long_term_weight: info.long_term_weight,
        })
    }

    async fn get_extended_block_header_in_range(
        &self,
        range: Range<u64>,
    ) -> Result<Vec<ExtendedBlockHeader>, tower::BoxError> {
        #[derive(Deserialize, Debug)]
        struct Response {
            headers: Vec<BlockInfo>,
        }

        let res = self
            .con
            .json_rpc_call::<Response>(
                "get_block_headers_range",
                Some(json!({"start_height": range.start, "end_height": range.end - 1})),
            )
            .await?;

        tracing::info!("Retrieved block headers in range: {:?}", range);

        Ok(rayon_spawn_async(|| {
            res.headers
                .into_iter()
                .map(|info| ExtendedBlockHeader {
                    version: HardFork::from_version(&info.major_version)
                        .expect("previously checked block has incorrect version"),
                    vote: HardFork::from_vote(&info.minor_version),
                    timestamp: info.timestamp,
                    cumulative_difficulty: u128_from_low_high(
                        info.cumulative_difficulty,
                        info.cumulative_difficulty_top64,
                    ),
                    block_weight: info.block_weight,
                    long_term_weight: info.long_term_weight,
                })
                .collect()
        })
        .await)
    }

    async fn get_blocks_in_range(
        &self,
        range: Range<u64>,
    ) -> Result<Vec<(Block, Vec<Transaction>)>, tower::BoxError> {
        tracing::info!("Getting blocks in range: {:?}", range);

        #[derive(Serialize)]
        pub struct Request {
            pub heights: Vec<u64>,
        }

        #[derive(Deserialize)]
        pub struct Response {
            pub blocks: Vec<BlockCompleteEntry>,
        }

        let res = self
            .con
            .bin_call(
                "get_blocks_by_height.bin",
                monero_epee_bin_serde::to_bytes(&Request {
                    heights: range.collect(),
                })?,
            )
            .await?;

        rayon_spawn_async(|| {
            let blocks: Response = monero_epee_bin_serde::from_bytes(res)?;

            blocks
                .blocks
                .into_par_iter()
                .map(|b| {
                    Ok((
                        Block::read(&mut b.block.as_slice())?,
                        match b.txs {
                            TransactionBlobs::Pruned(_) => {
                                return Err("node sent pruned txs!".into())
                            }
                            TransactionBlobs::Normal(txs) => txs
                                .into_par_iter()
                                .map(|tx| Transaction::read(&mut tx.as_slice()))
                                .collect::<Result<_, _>>()?,
                            TransactionBlobs::None => vec![],
                        },
                    ))
                })
                .collect::<Result<_, tower::BoxError>>()
        })
        .await
    }

    async fn get_outputs(
        &self,
        out_ids: HashMap<u64, HashSet<u64>>,
    ) -> Result<HashMap<u64, HashMap<u64, OutputOnChain>>, tower::BoxError> {
        tracing::info!(
            "Getting outputs len: {}",
            out_ids.values().map(|amt_map| amt_map.len()).sum::<usize>()
        );

        #[derive(Serialize, Copy, Clone)]
        struct OutputID {
            amount: u64,
            index: u64,
        }

        #[derive(Serialize, Clone)]
        struct Request<'a> {
            outputs: &'a [OutputID],
        }

        #[derive(Deserialize)]
        struct OutputRes {
            height: u64,
            key: [u8; 32],
            mask: [u8; 32],
            txid: [u8; 32],
        }

        #[derive(Deserialize)]
        struct Response {
            outs: Vec<OutputRes>,
        }

        let outputs = rayon_spawn_async(|| {
            out_ids
                .into_iter()
                .flat_map(|(amt, amt_map)| {
                    amt_map
                        .into_iter()
                        .map(|amt_idx| OutputID {
                            amount: amt,
                            index: amt_idx,
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .await;

        let res = self
            .con
            .bin_call(
                "get_outs.bin",
                monero_epee_bin_serde::to_bytes(&Request { outputs: &outputs })?,
            )
            .await?;

        let cache = self.cache.clone().read_owned().await;

        let span = tracing::Span::current();
        rayon_spawn_async(move || {
            let outs: Response = monero_epee_bin_serde::from_bytes(&res)?;

            tracing::info!(parent: &span, "Got outputs len: {}", outs.outs.len());

            let mut ret = HashMap::new();

            for (out, idx) in outs.outs.into_iter().zip(outputs) {
                ret.entry(idx.amount).or_insert_with(HashMap::new).insert(
                    idx.index,
                    OutputOnChain {
                        height: out.height,
                        time_lock: cache.outputs_time_lock(&out.txid),
                        // we unwrap these as we are checking already approved rings so if these points are bad
                        // then a bad proof has been approved.
                        key: CompressedEdwardsY::from_slice(&out.key)
                            .unwrap()
                            .decompress()
                            .unwrap(),
                        mask: CompressedEdwardsY::from_slice(&out.mask)
                            .unwrap()
                            .decompress()
                            .unwrap(),
                    },
                );
            }
            Ok(ret)
        })
        .await
    }

    async fn handle_request(
        &mut self,
        req: DatabaseRequest,
    ) -> Result<DatabaseResponse, tower::BoxError> {
        match req {
            DatabaseRequest::BlockHash(height) => {
                timeout(DEFAULT_TIMEOUT, self.get_block_hash(height))
                    .await?
                    .map(DatabaseResponse::BlockHash)
            }
            DatabaseRequest::ChainHeight => {
                let height = self.cache.read().await.height;

                let hash = timeout(DEFAULT_TIMEOUT, self.get_block_hash(height - 1)).await??;

                Ok(DatabaseResponse::ChainHeight(height, hash))
            }
            DatabaseRequest::BlockExtendedHeader(id) => {
                timeout(DEFAULT_TIMEOUT, self.get_extended_block_header(id))
                    .await?
                    .map(DatabaseResponse::BlockExtendedHeader)
            }
            DatabaseRequest::BlockExtendedHeaderInRange(range) => timeout(
                DEFAULT_TIMEOUT,
                self.get_extended_block_header_in_range(range),
            )
            .await?
            .map(DatabaseResponse::BlockExtendedHeaderInRange),
            DatabaseRequest::BlockBatchInRange(range) => {
                timeout(DEFAULT_TIMEOUT, self.get_blocks_in_range(range))
                    .await?
                    .map(DatabaseResponse::BlockBatchInRange)
            }
            DatabaseRequest::Outputs(out_ids) => {
                timeout(OUTPUTS_TIMEOUT, self.get_outputs(out_ids))
                    .await?
                    .map(DatabaseResponse::Outputs)
            }
            DatabaseRequest::NumberOutputsWithAmount(_)
            | DatabaseRequest::GeneratedCoins
            | DatabaseRequest::CheckKIsNotSpent(_) => {
                panic!("Request does not need RPC connection!")
            }
        }
    }

    #[instrument(level = "info", skip(self), fields(addr = self.address))]
    pub async fn check_rpc_alive(&self) -> Result<(), tower::BoxError> {
        tracing::debug!("Checking RPC connection");

        let res = timeout(Duration::from_secs(10), self.con.get_height()).await;
        let ok = matches!(res, Ok(Ok(_)));

        if !ok {
            tracing::warn!("RPC connection test failed");
            return Err("RPC connection test failed".into());
        }
        tracing::info!("RPC connection Ok");

        Ok(())
    }

    pub async fn run(mut self) {
        while let Some(req) = self.req_chan.next().await {
            let RpcReq {
                req,
                span,
                res_chan,
            } = req;

            let res = self.handle_request(req).instrument(span.clone()).await;

            let is_err = res.is_err();
            if is_err {
                tracing::warn!(parent: &span, "Error from RPC: {:?}", res)
            }

            let _ = res_chan.send(res);

            if is_err && self.check_rpc_alive().await.is_err() {
                break;
            }
        }

        tracing::warn!("Shutting down RPC connection: {}", self.address);

        self.req_chan.close();
        while let Some(req) = self.req_chan.try_next().unwrap() {
            let _ = req.res_chan.send(Err("RPC connection closed!".into()));
        }
    }
}

#[derive(Deserialize, Debug)]
struct BlockInfo {
    cumulative_difficulty: u64,
    cumulative_difficulty_top64: u64,
    timestamp: u64,
    block_weight: usize,
    long_term_weight: usize,

    major_version: u8,
    minor_version: u8,
}

fn u128_from_low_high(low: u64, high: u64) -> u128 {
    let res: u128 = high as u128;
    res << 64 | low as u128
}
