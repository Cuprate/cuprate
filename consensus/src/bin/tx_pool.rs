#![cfg(feature = "binaries")]

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::{channel::mpsc, FutureExt, StreamExt};
use monero_serai::transaction::Transaction;
use tokio::sync::oneshot;
use tower::{Service, ServiceExt};

use monero_consensus::{
    context::{BlockChainContext, BlockChainContextRequest, RawBlockChainContext},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    ConsensusError, TxNotInPool, TxPoolRequest, TxPoolResponse,
};

#[derive(Clone)]
pub struct TxPoolHandle {
    tx_pool_task: std::sync::Arc<tokio::task::JoinHandle<()>>,
    tx_pool_chan: mpsc::Sender<(
        TxPoolRequest,
        oneshot::Sender<Result<TxPoolResponse, TxNotInPool>>,
    )>,
}

impl tower::Service<TxPoolRequest> for TxPoolHandle {
    type Response = TxPoolResponse;
    type Error = TxNotInPool;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.tx_pool_task.is_finished() {
            panic!("Tx pool task finished before it was supposed to!");
        };

        self.tx_pool_chan
            .poll_ready(cx)
            .map_err(|_| panic!("Tx pool channel closed before it was supposed to"))
    }

    fn call(&mut self, req: TxPoolRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        self.tx_pool_chan
            .try_send((req, tx))
            .expect("You need to use `poll_ready` to check capacity!");

        async move {
            rx.await
                .expect("Tx pool will always respond without dropping the sender")
        }
        .boxed()
    }
}

pub type NewTxChanRec = mpsc::Receiver<(
    Vec<Transaction>,
    oneshot::Sender<Result<(), tower::BoxError>>,
)>;

pub type NewTxChanSen = mpsc::Sender<(
    Vec<Transaction>,
    oneshot::Sender<Result<(), tower::BoxError>>,
)>;

pub struct TxPool<TxV, Ctx> {
    txs: Arc<Mutex<HashMap<[u8; 32], Arc<TransactionVerificationData>>>>,
    current_ctx: BlockChainContext,
    tx_verifier: Option<TxV>,
    tx_verifier_chan: Option<oneshot::Receiver<TxV>>,
    ctx_svc: Ctx,
}

impl<TxV, Ctx> TxPool<TxV, Ctx>
where
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>
        + Clone
        + Send
        + 'static,
    TxV::Future: Send + 'static,
    Ctx: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Send
        + 'static,
    Ctx::Future: Send + 'static,
{
    pub async fn spawn(
        tx_verifier_chan: oneshot::Receiver<TxV>,
        mut ctx_svc: Ctx,
    ) -> Result<
        (
            TxPoolHandle,
            mpsc::Sender<(
                Vec<Transaction>,
                oneshot::Sender<Result<(), tower::BoxError>>,
            )>,
        ),
        tower::BoxError,
    > {
        let current_ctx = ctx_svc
            .ready()
            .await?
            .call(BlockChainContextRequest)
            .await?;

        let tx_pool = TxPool {
            txs: Default::default(),
            current_ctx,
            tx_verifier: None,
            tx_verifier_chan: Some(tx_verifier_chan),
            ctx_svc,
        };

        let (tx_pool_tx, tx_pool_rx) = mpsc::channel(3);
        let (new_tx_tx, new_tx_rx) = mpsc::channel(3);

        let tx_pool_task = tokio::spawn(tx_pool.run(tx_pool_rx, new_tx_rx));

        Ok((
            TxPoolHandle {
                tx_pool_task: tx_pool_task.into(),
                tx_pool_chan: tx_pool_tx,
            },
            new_tx_tx,
        ))
    }

    async fn get_or_update_ctx(&mut self) -> Result<RawBlockChainContext, tower::BoxError> {
        if let Ok(current_ctx) = self.current_ctx.blockchain_context().cloned() {
            Ok(current_ctx)
        } else {
            self.current_ctx = self
                .ctx_svc
                .ready()
                .await?
                .call(BlockChainContextRequest)
                .await?;
            self.current_ctx
                .blockchain_context()
                .map_err(Into::into)
                .cloned()
        }
    }

    fn handle_txs_req(
        &self,
        req: TxPoolRequest,
        tx: oneshot::Sender<Result<TxPoolResponse, TxNotInPool>>,
    ) {
        let TxPoolRequest::Transactions(txs_to_get) = req;

        let mut res = Vec::with_capacity(txs_to_get.len());

        let mut txs = self.txs.lock().unwrap();

        for tx_hash in txs_to_get {
            let Some(tx) = txs.remove(&tx_hash) else {
                let _ = tx.send(Err(TxNotInPool));
                return;
            };
            res.push(tx)
        }

        let _ = tx.send(Ok(TxPoolResponse::Transactions(res)));
    }

    async fn handle_new_txs(
        &mut self,
        new_txs: Vec<Transaction>,
        res_chan: oneshot::Sender<Result<(), tower::BoxError>>,
    ) -> Result<(), tower::BoxError> {
        if self.tx_verifier.is_none() {
            self.tx_verifier = Some(self.tx_verifier_chan.take().unwrap().await?);
        }

        let current_ctx = self.get_or_update_ctx().await?;

        let mut tx_verifier = self.tx_verifier.clone().unwrap();
        let tx_pool = self.txs.clone();

        tokio::spawn(async move {
            // We only batch the setup a real tx pool would also call `VerifyTxRequest::Block`
            let VerifyTxResponse::BatchSetupOk(txs) = tx_verifier
                .ready()
                .await
                .unwrap()
                .call(VerifyTxRequest::BatchSetup {
                    txs: new_txs,
                    hf: current_ctx.current_hard_fork,
                    re_org_token: current_ctx.re_org_token.clone(),
                })
                .await
                .unwrap()
            else {
                panic!("Tx verifier sent incorrect response!");
            };

            let mut locked_pool = tx_pool.lock().unwrap();

            for tx in txs {
                locked_pool.insert(tx.tx_hash, tx);
            }
            res_chan.send(Ok(())).unwrap();
        });
        Ok(())
    }

    pub async fn run(
        mut self,
        mut tx_pool_handle: mpsc::Receiver<(
            TxPoolRequest,
            oneshot::Sender<Result<TxPoolResponse, TxNotInPool>>,
        )>,
        mut new_tx_channel: NewTxChanRec,
    ) {
        loop {
            futures::select! {
                pool_req = tx_pool_handle.next() => {
                    let Some((req, tx)) = pool_req  else {
                        todo!("Shutdown txpool")
                    };
                    self.handle_txs_req(req, tx);
                }
                new_txs = new_tx_channel.next() => {
                    let Some(new_txs) = new_txs  else {
                        todo!("Shutdown txpool")
                    };

                    self.handle_new_txs(new_txs.0, new_txs.1).await.unwrap()
                }
            }
        }
    }
}

#[allow(dead_code)]
fn main() {}
