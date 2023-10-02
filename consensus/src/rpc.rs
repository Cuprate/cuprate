use std::cmp::min;
use std::future::Future;
use std::ops::Range;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::lock::{OwnedMutexGuard, OwnedMutexLockFuture};
use futures::{stream::FuturesOrdered, FutureExt, TryFutureExt, TryStreamExt};
use monero_serai::rpc::{HttpRpc, RpcConnection, RpcError};
use serde::Deserialize;
use serde_json::json;
use tower::balance::p2c::Balance;
use tower::util::BoxService;
use tower::ServiceExt;

use cuprate_common::BlockID;

use crate::block::pow::BlockPOWInfo;
use crate::block::weight::BlockWeightInfo;
use crate::hardforks::BlockHFInfo;
use crate::{DatabaseRequest, DatabaseResponse};

const MAX_BLOCKS_IN_RANGE: u64 = 50;

#[derive(Clone)]
pub struct Attempts(u64);

impl<Req: Clone, Res, E> tower::retry::Policy<Req, Res, E> for Attempts {
    type Future = futures::future::Ready<Self>;
    fn retry(&self, _: &Req, result: Result<&Res, &E>) -> Option<Self::Future> {
        if result.is_err() {
            // TODO:
            Some(futures::future::ready(Attempts(self.0)))
        } else {
            None
        }
    }

    fn clone_request(&self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}

pub fn init_rpc_load_balancer(
    addresses: Vec<String>,
) -> impl tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError> + Clone
{
    let rpc_discoverer = tower::discover::ServiceList::new(
        addresses
            .into_iter()
            .map(|addr| tower::load::Constant::new(Rpc::new_http(addr), 0)),
    );
    let rpc_balance = Balance::new(rpc_discoverer);
    let rpc_buffer = tower::buffer::Buffer::new(BoxService::new(rpc_balance), 3);
    let rpcs = tower::retry::Retry::new(Attempts(3), rpc_buffer);

    RpcBalancer { rpcs }
}

#[derive(Clone)]
pub struct RpcBalancer<T: Clone> {
    rpcs: T,
}

impl<T> tower::Service<DatabaseRequest> for RpcBalancer<T>
where
    T: tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>
        + Clone
        + Send
        + Sync
        + 'static,
    T::Future: Send + 'static,
{
    type Response = DatabaseResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DatabaseRequest) -> Self::Future {
        let this = self.rpcs.clone();

        match req {
            DatabaseRequest::BlockPOWInfoInRange(range) => {
                let resp_to_ret = |resp: DatabaseResponse| {
                    let DatabaseResponse::BlockPOWInfoInRange(pow_info) = resp else {
                        panic!("Database sent incorrect response");
                    };
                    pow_info
                };
                split_range_request(
                    this,
                    range,
                    DatabaseRequest::BlockPOWInfoInRange,
                    DatabaseResponse::BlockPOWInfoInRange,
                    resp_to_ret,
                )
            }

            DatabaseRequest::BlockWeightsInRange(range) => {
                let resp_to_ret = |resp: DatabaseResponse| {
                    let DatabaseResponse::BlockWeightsInRange(weights) = resp else {
                        panic!("Database sent incorrect response");
                    };
                    weights
                };
                split_range_request(
                    this,
                    range,
                    DatabaseRequest::BlockWeightsInRange,
                    DatabaseResponse::BlockWeightsInRange,
                    resp_to_ret,
                )
            }
            DatabaseRequest::BlockHfInfoInRange(range) => {
                let resp_to_ret = |resp: DatabaseResponse| {
                    let DatabaseResponse::BlockHfInfoInRange(hf_info) = resp else {
                        panic!("Database sent incorrect response");
                    };
                    hf_info
                };
                split_range_request(
                    this,
                    range,
                    DatabaseRequest::BlockHfInfoInRange,
                    DatabaseResponse::BlockHfInfoInRange,
                    resp_to_ret,
                )
            }
            req => this.oneshot(req).boxed(),
        }
    }
}

fn split_range_request<T, Ret>(
    rpc: T,
    range: Range<u64>,
    req: impl FnOnce(Range<u64>) -> DatabaseRequest + Clone + Send + 'static,
    resp: impl FnOnce(Vec<Ret>) -> DatabaseResponse + Send + 'static,
    resp_to_ret: impl Fn(DatabaseResponse) -> Vec<Ret> + Copy + Send + 'static,
) -> Pin<Box<dyn Future<Output = Result<DatabaseResponse, tower::BoxError>> + Send + 'static>>
where
    T: tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>
        + Clone
        + Send
        + Sync
        + 'static,
    T::Future: Send + 'static,
    Ret: Send + 'static,
{
    let iter = (0..range.clone().count() as u64)
        .step_by(MAX_BLOCKS_IN_RANGE as usize)
        .map(|i| {
            let req = req.clone();
            let new_range =
                (range.start + i)..(min(range.start + i + MAX_BLOCKS_IN_RANGE, range.end));
            rpc.clone().oneshot(req(new_range)).map_ok(resp_to_ret)
        });

    let fut = FuturesOrdered::from_iter(iter);

    let mut res = Vec::with_capacity(range.count());

    async move {
        for mut rpc_res in fut.try_collect::<Vec<Vec<_>>>().await?.into_iter() {
            res.append(&mut rpc_res)
        }

        Ok(resp(res))
    }
    .boxed()
}

enum RpcState<R: RpcConnection> {
    Locked,
    Acquiring(OwnedMutexLockFuture<monero_serai::rpc::Rpc<R>>),
    Acquired(OwnedMutexGuard<monero_serai::rpc::Rpc<R>>),
}
pub struct Rpc<R: RpcConnection> {
    rpc: Arc<futures::lock::Mutex<monero_serai::rpc::Rpc<R>>>,
    rpc_state: RpcState<R>,
    error_slot: Arc<Mutex<Option<RpcError>>>,
}

impl Rpc<HttpRpc> {
    pub fn new_http(addr: String) -> Rpc<HttpRpc> {
        let http_rpc = HttpRpc::new(addr).unwrap();
        Rpc {
            rpc: Arc::new(futures::lock::Mutex::new(http_rpc)),
            rpc_state: RpcState::Locked,
            error_slot: Arc::new(Mutex::new(None)),
        }
    }
}

impl<R: RpcConnection> Clone for Rpc<R> {
    fn clone(&self) -> Self {
        Rpc {
            rpc: Arc::clone(&self.rpc),
            rpc_state: RpcState::Locked,
            error_slot: Arc::clone(&self.error_slot),
        }
    }
}

impl<R: RpcConnection + Send + Sync + 'static> tower::Service<DatabaseRequest> for Rpc<R> {
    type Response = DatabaseResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(rpc_error) = self.error_slot.lock().unwrap().clone() {
            return Poll::Ready(Err(rpc_error.into()));
        }
        loop {
            match &mut self.rpc_state {
                RpcState::Locked => {
                    self.rpc_state = RpcState::Acquiring(Arc::clone(&self.rpc).lock_owned())
                }
                RpcState::Acquiring(rpc) => {
                    self.rpc_state = RpcState::Acquired(futures::ready!(rpc.poll_unpin(cx)))
                }
                RpcState::Acquired(_) => return Poll::Ready(Ok(())),
            }
        }
    }

    fn call(&mut self, req: DatabaseRequest) -> Self::Future {
        let RpcState::Acquired(rpc) = std::mem::replace(&mut self.rpc_state, RpcState::Locked)
        else {
            panic!("poll_ready was not called first!");
        };

        let err_slot = self.error_slot.clone();

        match req {
            DatabaseRequest::ChainHeight => async move {
                let res: Result<_, RpcError> = rpc
                    .get_height()
                    .map_ok(|height| DatabaseResponse::ChainHeight(height.try_into().unwrap()))
                    .await;
                if let Err(e) = &res {
                    *err_slot.lock().unwrap() = Some(e.clone());
                }
                res.map_err(Into::into)
            }
            .boxed(),

            DatabaseRequest::BlockPOWInfo(id) => get_blocks_pow_info(id, rpc).boxed(),
            DatabaseRequest::BlockWeights(id) => get_blocks_weight_info(id, rpc).boxed(),
            DatabaseRequest::BlockHFInfo(id) => get_blocks_hf_info(id, rpc).boxed(),
            DatabaseRequest::BlockHfInfoInRange(range) => {
                get_blocks_hf_info_in_range(range, rpc).boxed()
            }
            DatabaseRequest::BlockWeightsInRange(range) => {
                get_blocks_weight_info_in_range(range, rpc).boxed()
            }
            DatabaseRequest::BlockPOWInfoInRange(range) => {
                get_blocks_pow_info_in_range(range, rpc).boxed()
            }
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

async fn get_block_info_in_range<R: RpcConnection>(
    range: Range<u64>,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<Vec<BlockInfo>, tower::BoxError> {
    #[derive(Deserialize, Debug)]
    struct Response {
        headers: Vec<BlockInfo>,
    }

    let res = rpc
        .json_rpc_call::<Response>(
            "get_block_headers_range",
            Some(json!({"start_height": range.start, "end_height": range.end - 1})),
        )
        .await?;

    tracing::debug!("Retrieved blocks in range: {:?}", range);

    Ok(res.headers)
}

async fn get_block_info<R: RpcConnection>(
    id: BlockID,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<BlockInfo, tower::BoxError> {
    tracing::debug!("Retrieving block info with id: {}", id);

    #[derive(Deserialize, Debug)]
    struct Response {
        block_header: BlockInfo,
    }

    match id {
        BlockID::Height(height) => {
            let res = rpc
                .json_rpc_call::<Response>(
                    "get_block_header_by_height",
                    Some(json!({"height": height})),
                )
                .await?;
            Ok(res.block_header)
        }
        BlockID::Hash(hash) => {
            let res = rpc
                .json_rpc_call::<Response>("get_block_header_by_hash", Some(json!({"hash": hash})))
                .await?;
            Ok(res.block_header)
        }
    }
}

async fn get_blocks_weight_info_in_range<R: RpcConnection>(
    range: Range<u64>,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<DatabaseResponse, tower::BoxError> {
    let info = get_block_info_in_range(range, rpc).await?;

    Ok(DatabaseResponse::BlockWeightsInRange(
        info.into_iter()
            .map(|info| BlockWeightInfo {
                block_weight: info.block_weight,
                long_term_weight: info.long_term_weight,
            })
            .collect(),
    ))
}

async fn get_blocks_pow_info_in_range<R: RpcConnection>(
    range: Range<u64>,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<DatabaseResponse, tower::BoxError> {
    let info = get_block_info_in_range(range, rpc).await?;

    Ok(DatabaseResponse::BlockPOWInfoInRange(
        info.into_iter()
            .map(|info| BlockPOWInfo {
                timestamp: info.timestamp,
                cumulative_difficulty: u128_from_low_high(
                    info.cumulative_difficulty,
                    info.cumulative_difficulty_top64,
                ),
            })
            .collect(),
    ))
}

async fn get_blocks_weight_info<R: RpcConnection>(
    id: BlockID,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<DatabaseResponse, tower::BoxError> {
    let info = get_block_info(id, rpc).await?;

    Ok(DatabaseResponse::BlockWeights(BlockWeightInfo {
        block_weight: info.block_weight,
        long_term_weight: info.long_term_weight,
    }))
}

async fn get_blocks_pow_info<R: RpcConnection>(
    id: BlockID,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<DatabaseResponse, tower::BoxError> {
    let info = get_block_info(id, rpc).await?;

    Ok(DatabaseResponse::BlockPOWInfo(BlockPOWInfo {
        timestamp: info.timestamp,
        cumulative_difficulty: u128_from_low_high(
            info.cumulative_difficulty,
            info.cumulative_difficulty_top64,
        ),
    }))
}

fn u128_from_low_high(low: u64, high: u64) -> u128 {
    let res: u128 = high as u128;
    res << 64 | low as u128
}

async fn get_blocks_hf_info<R: RpcConnection>(
    id: BlockID,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<DatabaseResponse, tower::BoxError> {
    let info = get_block_info(id, rpc).await?;

    Ok(DatabaseResponse::BlockHfInfo(
        BlockHFInfo::from_major_minor(info.major_version, info.minor_version)?,
    ))
}

async fn get_blocks_hf_info_in_range<R: RpcConnection>(
    range: Range<u64>,
    rpc: OwnedMutexGuard<monero_serai::rpc::Rpc<R>>,
) -> Result<DatabaseResponse, tower::BoxError> {
    let info = get_block_info_in_range(range, rpc).await?;

    Ok(DatabaseResponse::BlockHfInfoInRange(
        info.into_iter()
            .map(|info| {
                BlockHFInfo::from_major_minor(info.major_version, info.minor_version).unwrap()
            })
            .collect(),
    ))
}
