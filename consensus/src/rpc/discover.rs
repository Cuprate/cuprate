use std::{sync::Arc, time::Duration};

use futures::{
    channel::mpsc::{self, SendError},
    stream::FuturesUnordered,
    SinkExt, StreamExt,
};
use monero_serai::rpc::HttpRpc;
use tokio::sync::RwLock;
use tower::{discover::Change, load::PeakEwma};
use tracing::instrument;

use super::{
    cache::ScanningCache,
    connection::{RpcConnection, RpcConnectionSvc},
};

#[instrument(skip(cache))]
async fn check_rpc(addr: String, cache: Arc<RwLock<ScanningCache>>) -> Option<RpcConnectionSvc> {
    tracing::debug!("Sending request to node.");

    let con = HttpRpc::with_custom_timeout(addr.clone(), Duration::from_secs(u64::MAX))
        .await
        .ok()?;
    let (tx, rx) = mpsc::channel(0);
    let rpc = RpcConnection {
        address: addr.clone(),
        con,
        cache,
        req_chan: rx,
    };

    rpc.check_rpc_alive().await.ok()?;
    let handle = tokio::spawn(rpc.run());

    Some(RpcConnectionSvc {
        address: addr,
        rpc_task_chan: tx,
        rpc_task_handle: handle,
    })
}

pub(crate) struct RPCDiscover {
    pub initial_list: Vec<String>,
    pub ok_channel: mpsc::Sender<Change<usize, PeakEwma<RpcConnectionSvc>>>,
    pub already_connected: usize,
    pub cache: Arc<RwLock<ScanningCache>>,
}

impl RPCDiscover {
    async fn found_rpc(&mut self, rpc: RpcConnectionSvc) -> Result<(), SendError> {
        self.already_connected += 1;

        self.ok_channel
            .send(Change::Insert(
                self.already_connected,
                PeakEwma::new(
                    rpc,
                    Duration::from_secs(5000),
                    3000.0,
                    tower::load::CompleteOnResponse::default(),
                ),
            ))
            .await?;

        Ok(())
    }

    pub async fn run(mut self) {
        if !self.initial_list.is_empty() {
            let mut fut = FuturesUnordered::from_iter(
                self.initial_list
                    .drain(..)
                    .map(|addr| check_rpc(addr, self.cache.clone())),
            );

            while let Some(res) = fut.next().await {
                if let Some(rpc) = res {
                    if self.found_rpc(rpc).await.is_err() {
                        tracing::info!("Stopping RPC discover channel closed!");
                        return;
                    }
                }
            }
        }
    }
}
