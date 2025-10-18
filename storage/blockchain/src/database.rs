use std::{future::pending, sync::RwLock};
use std::mem;
use crate::types::RctOutput;
use cuprate_database::{ConcreteEnv, DbResult, Env, InitError, RuntimeError};
use cuprate_linear_tape::LinearTape;
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use rayon::ThreadPool;
//use tokio::sync::{ OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use futures::channel::oneshot;
use tokio_util::sync::ReusableBoxFuture;
use tower::Service;
use crate::config::{init_thread_pool, Config};
use crate::service::{map_read_request, map_write_request};

pub struct BlockchainDatabase<E: Env> {
    pub(crate) dynamic_tables: E,
    pub(crate) rct_outputs: LinearTape<RctOutput>,
}

pub struct BlockchainDatabaseService<E: Env + 'static> {
    pool: Arc<ThreadPool>,

    database: Arc<RwLock<BlockchainDatabase<E>>>,

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

        let database = crate::free::open(config)?;

        Ok(Self {
            pool,
            database: Arc::new(RwLock::new(database)),
          //  lock_state: LockState::Waiting(None, None),
        })

    }

    pub fn init_with_pool(config: Config, pool: Arc<ThreadPool>) -> Result<Self, InitError> {
        let database = crate::free::open(config)?;

        Ok(Self {
            pool,
            database: Arc::new(RwLock::new(database)),
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
            drop(response_sender.send(map_read_request(&database.read().unwrap(), req)));
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
            drop(response_sender.send(map_write_request(&mut database.write().unwrap(), &req)));
        });

        InfallibleOneshotReceiver::from(receiver)
    }
}
/*
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