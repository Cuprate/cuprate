use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    future::Future,
    ops::Range,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    stream::{FuturesOrdered, FuturesUnordered},
    FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};
use tokio::sync::RwLock;
use tower::{balance::p2c::Balance, ServiceExt};

use crate::{helper::rayon_spawn_async, DatabaseRequest, DatabaseResponse};

pub mod cache;
mod connection;
mod discover;

use cache::ScanningCache;

const MAX_OUTS_PER_RPC: usize = 5000; // the cap for monerod is 5000

#[derive(Debug, Copy, Clone)]
pub struct RpcConfig {
    pub max_blocks_per_node: u64,
    pub max_block_headers_per_node: u64,
}

impl RpcConfig {
    pub fn block_batch_size(&self) -> u64 {
        self.max_blocks_per_node * 3
    }

    pub fn new(max_blocks_per_node: u64, max_block_headers_per_node: u64) -> RpcConfig {
        RpcConfig {
            max_block_headers_per_node,
            max_blocks_per_node,
        }
    }
}

#[derive(Clone)]
pub struct Attempts(u64);

impl<Req: Clone, Res, E> tower::retry::Policy<Req, Res, E> for Attempts {
    type Future = futures::future::Ready<Self>;
    fn retry(&self, _: &Req, result: Result<&Res, &E>) -> Option<Self::Future> {
        if result.is_err() {
            if self.0 == 0 {
                None
            } else {
                Some(futures::future::ready(Attempts(self.0 - 1)))
            }
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
    cache: Arc<RwLock<ScanningCache>>,
    config: Arc<std::sync::RwLock<RpcConfig>>,
) -> impl tower::Service<
    DatabaseRequest,
    Response = DatabaseResponse,
    Error = tower::BoxError,
    Future = Pin<
        Box<dyn Future<Output = Result<DatabaseResponse, tower::BoxError>> + Send + 'static>,
    >,
> + Clone {
    let (rpc_discoverer_tx, rpc_discoverer_rx) = futures::channel::mpsc::channel(0);

    let rpc_balance = Balance::new(Box::pin(
        rpc_discoverer_rx.map(Result::<_, tower::BoxError>::Ok),
    ));
    let rpc_buffer = tower::buffer::Buffer::new(rpc_balance, 50);
    let rpcs = tower::retry::Retry::new(Attempts(10), rpc_buffer);

    let discover = discover::RPCDiscover {
        initial_list: addresses,
        ok_channel: rpc_discoverer_tx,
        already_connected: Default::default(),
        cache: cache.clone(),
    };

    tokio::spawn(discover.run());

    RpcBalancer {
        rpcs,
        config,
        cache,
    }
}

#[derive(Clone)]
pub struct RpcBalancer<T: Clone> {
    rpcs: T,
    config: Arc<std::sync::RwLock<RpcConfig>>,
    cache: Arc<RwLock<ScanningCache>>,
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
        let config_mutex = self.config.clone();
        let config = config_mutex.read().unwrap();

        let cache = self.cache.clone();

        match req {
            DatabaseRequest::CheckKIsNotSpent(kis) => async move {
                Ok(DatabaseResponse::CheckKIsNotSpent(
                    cache.read().await.are_kis_spent(kis),
                ))
            }
            .boxed(),
            DatabaseRequest::GeneratedCoins => async move {
                Ok(DatabaseResponse::GeneratedCoins(
                    cache.read().await.already_generated_coins,
                ))
            }
            .boxed(),
            DatabaseRequest::NumberOutputsWithAmount(amt) => async move {
                Ok(DatabaseResponse::NumberOutputsWithAmount(
                    cache.read().await.numb_outs(amt),
                ))
            }
            .boxed(),
            DatabaseRequest::BlockBatchInRange(range) => {
                let resp_to_ret = |resp: DatabaseResponse| {
                    let DatabaseResponse::BlockBatchInRange(pow_info) = resp else {
                        panic!("Database sent incorrect response");
                    };
                    pow_info
                };
                split_range_request(
                    this,
                    range,
                    DatabaseRequest::BlockBatchInRange,
                    DatabaseResponse::BlockBatchInRange,
                    resp_to_ret,
                    config.max_blocks_per_node,
                )
                .boxed()
            }
            DatabaseRequest::BlockExtendedHeaderInRange(range) => {
                let resp_to_ret = |resp: DatabaseResponse| {
                    let DatabaseResponse::BlockExtendedHeaderInRange(pow_info) = resp else {
                        panic!("Database sent incorrect response");
                    };
                    pow_info
                };
                split_range_request(
                    this,
                    range,
                    DatabaseRequest::BlockExtendedHeaderInRange,
                    DatabaseResponse::BlockExtendedHeaderInRange,
                    resp_to_ret,
                    config.max_block_headers_per_node,
                )
                .boxed()
            }
            DatabaseRequest::Outputs(outs) => async move {
                let split_outs = rayon_spawn_async(|| {
                    let mut split_outs: Vec<HashMap<u64, HashSet<u64>>> = Vec::new();
                    let mut i: usize = 0;
                    for (amount, ixs) in outs {
                        if ixs.len() > MAX_OUTS_PER_RPC {
                            for ii in (0..ixs.len()).step_by(MAX_OUTS_PER_RPC) {
                                let mut amt_map = HashSet::with_capacity(MAX_OUTS_PER_RPC);
                                amt_map.extend(ixs.iter().skip(ii).copied().take(MAX_OUTS_PER_RPC));

                                let mut map = HashMap::new();
                                map.insert(amount, amt_map);
                                split_outs.push(map);
                                i += 1;
                            }
                            continue;
                        }

                        if let Some(map) = split_outs.get_mut(i.saturating_sub(1)) {
                            if map.iter().map(|(_, amt_map)| amt_map.len()).sum::<usize>()
                                + ixs.len()
                                < MAX_OUTS_PER_RPC
                            {
                                assert!(map.insert(amount, ixs).is_none());
                                continue;
                            }
                        }
                        let mut map = HashMap::new();
                        map.insert(amount, ixs);
                        split_outs.push(map);
                        i += 1;
                    }
                    split_outs
                })
                .await;

                let mut futs = FuturesUnordered::from_iter(
                    split_outs
                        .into_iter()
                        .map(|map| this.clone().oneshot(DatabaseRequest::Outputs(map))),
                );

                let mut outs = HashMap::new();

                while let Some(out_response) = futs.next().await {
                    let DatabaseResponse::Outputs(out_response) = out_response? else {
                        panic!("RPC sent incorrect response!");
                    };
                    out_response.into_iter().for_each(|(amt, amt_map)| {
                        outs.entry(amt).or_insert_with(HashMap::new).extend(amt_map)
                    });
                }
                Ok(DatabaseResponse::Outputs(outs))
            }
            .boxed(),
            req => this.oneshot(req).boxed(),
        }
    }
}

fn split_range_request<T, Ret>(
    rpc: T,
    range: Range<u64>,
    req: impl Fn(Range<u64>) -> DatabaseRequest + Send + 'static,
    resp: impl FnOnce(Vec<Ret>) -> DatabaseResponse + Send + 'static,
    resp_to_ret: impl Fn(DatabaseResponse) -> Vec<Ret> + Copy + Send + 'static,
    max_request_per_rpc: u64,
) -> impl Future<Output = Result<DatabaseResponse, tower::BoxError>> + Send + 'static
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
        .step_by(max_request_per_rpc as usize)
        .map(|i| {
            let new_range =
                (range.start + i)..(min(range.start + i + max_request_per_rpc, range.end));
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
}
