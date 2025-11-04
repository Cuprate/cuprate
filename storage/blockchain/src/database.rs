use crate::types::{RctOutput, TxInfo};
use cuprate_database::DatabaseRo;
use cuprate_database::{ConcreteEnv, DbResult, Env, EnvInner, InitError, RuntimeError};
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_linear_tape::{Flush, LinearBlobTapeAppender, LinearTapeAppender, LinearTapes};
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};
use rayon::ThreadPool;
use std::mem;
use std::sync::{Arc, RwLockReadGuard};
use std::task::{ready, Context, Poll};
use parking_lot::RwLock;
use std::iter::once;
//use tokio::sync::{ OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use crate::config::{init_thread_pool, Config};
use crate::service::{map_read_request, map_write_request};
use futures::channel::oneshot;
use tokio_util::sync::ReusableBoxFuture;
use tower::Service;

pub const RCT_OUTPUTS: &str = "rct_outputs";
pub const PRUNED_BLOBS: &str = "pruned_blobs";
pub const PRUNABLE_BLOBS: [&str; 8] = ["prunable1", "prunable2", "prunable3", "prunable4", "prunable5", "prunable6", "prunable7", "prunable8"];

pub const TX_INFOS: &str = "tx_infos";
pub const BLOCK_INFOS: &str = "block_infos";

pub struct BlockchainDatabase<E: Env> {
    pub(crate) dynamic_tables: E,
    pub(crate) tapes: LinearTapes,
}

pub struct BlockchainDatabaseService<E: Env + 'static> {
    pool: Arc<ThreadPool>,

    database: Arc<BlockchainDatabase<E>>,
    //    lock_state: LockState<E>
}

impl<E: Env> Clone for BlockchainDatabaseService<E> {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            database: Arc::clone(&self.database),
            //  lock_state: LockState::Waiting(None, None),
        }
    }
}

impl<E: Env + 'static> BlockchainDatabaseService<E> {
    pub fn init(config: Config) -> Result<Self, InitError> {
        let pool = init_thread_pool(config.reader_threads);

        let mut database = crate::free::open(config)?;
        //check_rct_output_tape_consistency(&mut database);

        Ok(Self {
            pool,
            database: Arc::new(database),
            //  lock_state: LockState::Waiting(None, None),
        })
    }

    pub fn init_with_pool(config: Config, pool: Arc<ThreadPool>) -> Result<Self, InitError> {
        let mut database = crate::free::open(config)?;
        //check_rct_output_tape_consistency(&mut database);

        Ok(Self {
            pool,
            database: Arc::new(database),
            //  lock_state: LockState::Waiting(None, None),
        })
    }

    pub fn disarm(&mut self) {
        //self.lock_state.disarm()
    }
}

impl<E: Env + 'static> Service<BlockchainReadRequest> for BlockchainDatabaseService<E> {
    type Response = BlockchainResponse;
    type Error = RuntimeError;
    type Future = InfallibleOneshotReceiver<DbResult<Self::Response>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        //self.lock_state.poll_read(cx, &self.database).map(Ok)
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BlockchainReadRequest) -> Self::Future {
        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        let database = self.database.clone(); // self.lock_state.take_read();

        // Spawn the request in the rayon DB thread-pool.
        //
        // Note that this uses `self.pool` instead of `rayon::spawn`
        // such that any `rayon` parallel code that runs within
        // the passed closure uses the same `rayon` threadpool.
        self.pool.spawn(move || {
            drop(response_sender.send(map_read_request(&database, req)));
        });

        InfallibleOneshotReceiver::from(receiver)
    }
}

impl<E: Env + 'static> Service<BlockchainWriteRequest> for BlockchainDatabaseService<E> {
    type Response = BlockchainResponse;
    type Error = RuntimeError;
    type Future = InfallibleOneshotReceiver<DbResult<Self::Response>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        //self.lock_state.poll_write(cx, &self.database).map(Ok)
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BlockchainWriteRequest) -> Self::Future {
        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        let mut database = self.database.clone();

        // Spawn the request in the rayon DB thread-pool.
        //
        // Note that this uses `self.pool` instead of `rayon::spawn`
        // such that any `rayon` parallel code that runs within
        // the passed closure uses the same `rayon` threadpool.
        self.pool.spawn(move || {
            drop(response_sender.send(map_write_request(&database, &req)));
        });

        InfallibleOneshotReceiver::from(receiver)
    }
}

/*
fn check_rct_output_tape_consistency<E: Env>(blockchain_database: &mut BlockchainDatabase<E>) {
    let env_inner = blockchain_database.dynamic_tables.env_inner();

    let tx_ro = env_inner.tx_ro().unwrap();

    let block_infos = env_inner.open_db_ro::<BlockInfos>(&tx_ro).unwrap();
    let Some(top_block) = block_infos.len().unwrap().checked_sub(1) else {
        return;
    };

    let top_block_info = block_infos.get(&(top_block as usize)).unwrap();

    let mut tapes = blockchain_database.tapes.write();
    let mut rct_tape = &mut tapes.rct_outputs;
    if top_block_info.cumulative_rct_outs < rct_tape.reader().unwrap().len() as u64 {
        let amt_to_pop = rct_tape.reader().unwrap().len() as u64 - top_block_info.cumulative_rct_outs;
        let mut popper = rct_tape.popper();

        popper.pop_entries(amt_to_pop as usize);
        popper.flush(Flush::Sync).unwrap();
    } else if top_block_info.cumulative_rct_outs > rct_tape.reader().unwrap().len() as u64 {
        todo!()
    }
}

enum LockState<E: Env> {
    Waiting(
        Option<
            ReusableBoxFuture<
                'static,
                OwnedRwLockReadGuard<BlockchainDatabase<E>>,
            >,
        >,
        Option<
            ReusableBoxFuture<
                'static,
                OwnedRwLockWriteGuard<BlockchainDatabase<E>>,
            >,
        >,
    ),
    PendingRead(
        ReusableBoxFuture<
            'static,
            OwnedRwLockReadGuard<BlockchainDatabase<E>>,
        >,
    ),
    PendingWrite(
        ReusableBoxFuture<
            'static,
            OwnedRwLockWriteGuard<BlockchainDatabase<E>>,
        >,
    ),
    LockedRead(ReusableBoxFuture<
        'static,
        OwnedRwLockReadGuard<BlockchainDatabase<E>>,
    >, OwnedRwLockReadGuard<BlockchainDatabase<E>>),
    LockedWrite(ReusableBoxFuture<
        'static,
        OwnedRwLockWriteGuard<BlockchainDatabase<E>>,
    >, OwnedRwLockWriteGuard<BlockchainDatabase<E>>),
}

impl<E: Env + 'static> LockState<E> {
    fn disarm(&mut self) {
        match mem::replace(self, LockState::Waiting(None, None)) {
            Self::Waiting(read, write) =>  {
                *self = Self::Waiting(read, write);
                return;
            },
            LockState::PendingRead(read_fut) | LockState::LockedRead(read_fut, _)=> {
                *self = Self::Waiting(Some(read_fut), None);
                return;
            },
            LockState::PendingWrite(write_fut) | LockState::LockedWrite(write_fut, _) => {
                *self = Self::Waiting(None, Some(write_fut));
                return;
            }
        }
    }
    fn take_write(&mut self) -> OwnedRwLockWriteGuard<BlockchainDatabase<E>> {
        match mem::replace(self, LockState::Waiting(None, None)) {
            LockState::LockedWrite(write_fut, write) => {


                *self = LockState::Waiting(None, Some(write_fut));
                write
            }
            _ => {
                panic!("poll_ready was not called first");
            }
        }
    }

    fn take_read(&mut self) -> OwnedRwLockReadGuard<BlockchainDatabase<E>> {
        match mem::replace(self, LockState::Waiting(None, None)) {
            LockState::LockedRead(read_fut, read) => {

                *self = LockState::Waiting(Some(read_fut), None);
                read
            }
            _ => {
                panic!("poll_ready was not called first");
            }
        }
    }

    fn poll_read(
        &mut self,
        cx: &mut Context<'_>,
        database: &Arc<RwLock<BlockchainDatabase<E>>>,
    ) -> Poll<()> {
        loop {
            match mem::replace(self, LockState::Waiting(None, None)) {
                LockState::Waiting(read, _) => {
                    let mut read = read
                        .unwrap_or_else(|| ReusableBoxFuture::new(pending()));

                    read.set(database.clone().read_owned());

                    *self = LockState::PendingRead(read);
                }
                LockState::PendingRead(mut read_fut) => {
                    return match read_fut.poll(cx) {
                        Poll::Ready(read) => {
                            *self = LockState::LockedRead(read_fut, read);
                            Poll::Ready(())
                        }
                        Poll::Pending => {
                            *self = LockState::PendingRead(read_fut);
                            Poll::Pending
                        }
                    }
                }
                LockState::PendingWrite(_) | LockState::LockedWrite(_, _) => {
                    let read = ReusableBoxFuture::new(database.clone().read_owned());

                    *self = LockState::PendingRead(read);
                }
                LockState::LockedRead(read_fut, read) => {
                    *self = LockState::LockedRead(read_fut, read);
                    return Poll::Ready(());
                }
            }
        }
    }

    fn poll_write(
        &mut self,
        cx: &mut Context<'_>,
        database: &Arc<RwLock<BlockchainDatabase<E>>>,
    ) -> Poll<()> {
        loop {
            match mem::replace(self, LockState::Waiting(None, None)) {
                LockState::Waiting(_, write) => {
                    let mut write = write
                        .unwrap_or_else(|| ReusableBoxFuture::new(pending()));

                    write.set(database.clone().write_owned());

                    *self = LockState::PendingWrite(write);
                }
                LockState::PendingWrite(mut write_fut) => {
                    return match write_fut.poll(cx) {
                        Poll::Ready(write) => {
                            *self = LockState::LockedWrite(write_fut, write);
                            return Poll::Ready(());
                        }
                        Poll::Pending => {
                            *self = LockState::PendingWrite(write_fut);
                            Poll::Pending
                        }
                    }
                }
                LockState::PendingRead(_) | LockState::LockedRead(_, _) => {
                    let write = ReusableBoxFuture::new(database.clone().write_owned());

                    *self = LockState::PendingWrite(write);
                }
                LockState::LockedWrite(write_fut, write) => {
                    *self = LockState::LockedWrite(write_fut, write);

                    return Poll::Ready(());
                }
            }
        }
    }
}

 */
