//! Database writer thread definitions and logic.

use std::{
    borrow::Cow,
    sync::Arc,
    task::{Context, Poll},
};

use crossbeam::channel::Receiver;
use fjall::PersistMode;
use futures::channel::oneshot;
use tapes::Persistence;
use tapes::TapesRead;
use tower::Service;
use tracing::instrument;

use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, ChainId, VerifiedBlockInformation,
};

use crate::{
    error::{BlockchainError, DbResult},
    ops::block::add_blocks_to_tapes,
    service::ResponseResult,
    BlockchainDatabase,
};

//---------------------------------------------------------------------------------------------------- init_write_service
/// Initialise the blockchain write service from a [`BlockchainDatabase`].
pub fn init_write_service(env: Arc<BlockchainDatabase>) -> BlockchainWriteHandle {
    let (sender, receiver) = crossbeam::channel::unbounded();

    std::thread::Builder::new()
        .name("cuprate_blockchain_writer".into())
        .spawn(move || writer_thread(&env, &receiver))
        .unwrap();

    BlockchainWriteHandle { sender }
}

/// The [`tower::Service`] handle to write to the database.
pub struct BlockchainWriteHandle {
    /// Sender channel to the database write thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    sender: crossbeam::channel::Sender<(
        BlockchainWriteRequest,
        oneshot::Sender<DbResult<BlockchainResponse>>,
    )>,
}

impl Service<BlockchainWriteRequest> for BlockchainWriteHandle {
    type Response = BlockchainResponse;
    type Error = BlockchainError;
    type Future = cuprate_helper::asynch::InfallibleOneshotReceiver<DbResult<BlockchainResponse>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BlockchainWriteRequest) -> Self::Future {
        let (response_sender, receiver) = oneshot::channel();

        self.sender.try_send((req, response_sender)).unwrap();

        cuprate_helper::asynch::InfallibleOneshotReceiver::from(receiver)
    }
}

#[instrument(
    name = "blockchain_writer_thread",
    skip(env, receiver),
    level = "error"
)]
fn writer_thread(
    env: &Arc<BlockchainDatabase>,
    receiver: &Receiver<(
        BlockchainWriteRequest,
        oneshot::Sender<DbResult<BlockchainResponse>>,
    )>,
) {
    while let Ok((req, response_sender)) = receiver.recv() {
        let span = tracing::debug_span!("write_request");
        span.in_scope(|| {
            let response = handle_blockchain_request(env, &req);

            match &response {
                Ok(_) => tracing::debug!("Sending successful write response."),
                Err(e) => {
                    tracing::error!("Failed to handle write request: {e:?}");
                }
            }

            let _ = response_sender.send(response).inspect_err(|_| {
                tracing::warn!("Failed to send write response, rx wasn't waiting.");
            });
        });
    }
}

//---------------------------------------------------------------------------------------------------- handle_bc_request
/// Handle an incoming [`BlockchainWriteRequest`], returning a [`BlockchainResponse`].
fn handle_blockchain_request(
    env: &Arc<BlockchainDatabase>,
    req: &BlockchainWriteRequest,
) -> Result<BlockchainResponse, BlockchainError> {
    match req {
        BlockchainWriteRequest::WriteBlock(block) => write_block(env, block),
        BlockchainWriteRequest::BatchWriteBlocks(blocks) => write_blocks(env, blocks),
        BlockchainWriteRequest::WriteAltBlock(alt_block) => write_alt_block(env, alt_block),
        BlockchainWriteRequest::PopBlocks(numb_blocks) => pop_blocks(env, *numb_blocks),
        BlockchainWriteRequest::FlushAltBlocks => flush_alt_blocks(env),
    }
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`Request`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `BlockExtendedHeader` -> `block_extended_header`.
//
// Each function will return the [`Response`] that we
// should send back to the caller in [`map_request()`].

/// [`BlockchainWriteRequest::WriteBlock`].
#[inline]
#[instrument(skip(db, block), level = "debug")]
fn write_block(db: &BlockchainDatabase, block: &VerifiedBlockInformation) -> ResponseResult {
    write_blocks(db, std::slice::from_ref(block))
}

/// [`BlockchainWriteRequest::BatchWriteBlocks`].
#[inline]
#[instrument(skip(db, blocks), level = "debug")]
fn write_blocks(db: &BlockchainDatabase, blocks: &[VerifiedBlockInformation]) -> ResponseResult {
    tracing::debug!("Writing {} block(s) to database.", blocks.len());

    let mut tapes = db.linear_tapes.append();

    let numb_transactions = tapes
        .fixed_sized_tape_len(&db.tx_infos)
        .expect("required tape not open");

    add_blocks_to_tapes(blocks, db, &mut tapes)?;

    let mut tapes = Some(tapes);

    let mut pre_rct_numb_outputs_cache = db.pre_rct_numb_outputs_cache.lock().unwrap();

    let mut result = move || {
        let mut numb_transactions = numb_transactions;

        let mut tx_rw = db.fjall.batch().durability(Some(PersistMode::Buffer));

        for block in blocks {
            crate::ops::block::add_block_to_dynamic_tables(
                db,
                &block.block,
                &block.block_hash,
                block.txs.iter().map(|tx| Cow::Borrowed(&tx.tx)),
                &mut numb_transactions,
                &mut tx_rw,
                &mut pre_rct_numb_outputs_cache,
            )?;
        }

        if let Some(mut tapes) = tapes.take() {
            tapes.commit(Persistence::Buffer)?;
        }

        tx_rw.commit().unwrap();

        Ok(BlockchainResponse::Ok)
    };

    result()
}

/// [`BlockchainWriteRequest::WriteAltBlock`].
#[inline]
fn write_alt_block(db: &BlockchainDatabase, block: &AltBlockInformation) -> ResponseResult {
    let mut tx_rw = db.fjall.batch();

    crate::ops::alt_block::add_alt_block(db, block, &mut tx_rw)?;

    tx_rw.commit()?;

    Ok(BlockchainResponse::Ok)
}

/// [`BlockchainWriteRequest::PopBlocks`].
fn pop_blocks(db: &BlockchainDatabase, numb_blocks: usize) -> ResponseResult {
    let mut tapes = db.linear_tapes.truncate();
    let mut tx_rw = db.fjall.batch();

    // flush all the current alt blocks as they may reference blocks to be popped.
    crate::ops::alt_block::flush_alt_blocks(db)?;

    // generate a `ChainId` for the popped blocks.
    let old_main_chain_id = ChainId(rand::random());

    // pop the blocks
    for _ in 0..numb_blocks {
        crate::ops::block::pop_block(db, Some(old_main_chain_id), &mut tx_rw, &mut tapes)?;
    }

    tx_rw.commit()?;
    tapes.commit(Persistence::SyncAll)?;
    Ok(BlockchainResponse::PopBlocks(old_main_chain_id))
}

/// [`BlockchainWriteRequest::FlushAltBlocks`].
#[inline]
fn flush_alt_blocks(db: &BlockchainDatabase) -> ResponseResult {
    crate::ops::alt_block::flush_alt_blocks(db)?;

    Ok(BlockchainResponse::Ok)
}
