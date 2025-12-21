//! Database writer thread definitions and logic.

use std::net::Shutdown::Read;
//---------------------------------------------------------------------------------------------------- Import
use cuprate_database_service::{DatabaseWriteHandle, RuntimeError};
use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, ChainId, VerifiedBlockInformation,
};
use heed::MdbError;
use std::sync::Arc;
use fjall::PersistMode;
use tapes::Flush;

use crate::database::TX_INFOS;
use crate::error::{BlockchainError, DbResult};
use crate::ops::block::add_blocks_to_tapes;
use crate::types::TxInfo;
use crate::{
    service::types::{BlockchainWriteHandle, ResponseResult},
    Blockchain,
};

/// Write functions within this module abort if the write transaction
/// could not be aborted successfully to maintain atomicity.
///
/// This is the panic message if the `abort()` fails.
const TX_RW_ABORT_FAIL: &str =
    "Could not maintain blockchain database atomicity by aborting write transaction";

//---------------------------------------------------------------------------------------------------- init_write_service
/// Initialize the blockchain write service from a [`ConcreteEnv`].
pub fn init_write_service(env: Arc<Blockchain>) -> BlockchainWriteHandle {
    DatabaseWriteHandle::init(env, handle_blockchain_request)
}

//---------------------------------------------------------------------------------------------------- handle_bc_request
/// Handle an incoming [`BlockchainWriteRequest`], returning a [`BlockchainResponse`].
fn handle_blockchain_request(
    env: &Arc<Blockchain>,
    req: &BlockchainWriteRequest,
) -> Result<BlockchainResponse, RuntimeError> {
    Ok(match req {
        BlockchainWriteRequest::WriteBlock(block) => write_block(env, block),
        BlockchainWriteRequest::BatchWriteBlocks(blocks) => write_blocks(env, blocks),
        BlockchainWriteRequest::WriteAltBlock(alt_block) => write_alt_block(env, alt_block),
        BlockchainWriteRequest::PopBlocks(numb_blocks) => pop_blocks(env, *numb_blocks),
        BlockchainWriteRequest::FlushAltBlocks => flush_alt_blocks(env),
    }
    .map_err(|e| RuntimeError::Io(std::io::Error::other(e)))?)
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
fn write_block(db: &Blockchain, block: &VerifiedBlockInformation) -> ResponseResult {
    write_blocks(db, std::slice::from_ref(block))
}

/// [`BlockchainWriteRequest::BatchWriteBlocks`].
#[inline]
fn write_blocks(db: &Blockchain, blocks: &[VerifiedBlockInformation]) -> ResponseResult {
    let mut tapes = db.linear_tapes.appender();
    let numb_transactions = tapes.fixed_sized_tape_appender::<TxInfo>(TX_INFOS).len();
    add_blocks_to_tapes(blocks, &mut tapes)?;

    let mut tapes = Some(tapes);

    let mut result = move || {
        let mut numb_transactions = numb_transactions;

        let mut tx_rw = db.fjall_keyspace.write_tx();

        for block in blocks {
            crate::ops::block::add_block_to_dynamic_tables_fjall(
                db,
                block,
                &mut numb_transactions,
                &mut tx_rw,
            )?;
        }

        if let Some(tapes) = tapes.take() {
            tapes.flush(Flush::Async)?;
        }

        tx_rw.commit().unwrap();
        db.fjall_keyspace.persist(PersistMode::Buffer).unwrap();

        Ok(BlockchainResponse::Ok)
    };

    loop {
        let result = result();

        if matches!(
            result,
            Err(BlockchainError::Heed(heed::Error::Mdb(MdbError::MapFull)))
        ) {
            todo!();
            continue;
        }

        return result;
    }
}

/// [`BlockchainWriteRequest::WriteAltBlock`].
#[inline]
fn write_alt_block(db: &Blockchain, block: &AltBlockInformation) -> ResponseResult {
    let result = || {
        let mut tx_rw = db.dynamic_tables.write_txn()?;

        crate::ops::alt_block::add_alt_block(db, block, &mut tx_rw)?;

        tx_rw.commit()?;

        Ok(BlockchainResponse::Ok)
    };

    loop {
        let result = result();

        if matches!(
            result,
            Err(BlockchainError::Heed(heed::Error::Mdb(MdbError::MapFull)))
        ) {
            todo!();
            continue;
        }

        return result;
    }
}

/// [`BlockchainWriteRequest::PopBlocks`].
fn pop_blocks(db: &Blockchain, numb_blocks: usize) -> ResponseResult {
    // FIXME: turn this function into a try block once stable.
    let mut result = || {
        let mut tx_rw = db.dynamic_tables.write_txn()?;
        let mut tapes = db.linear_tapes.popper();

        // flush all the current alt blocks as they may reference blocks to be popped.
        crate::ops::alt_block::flush_alt_blocks(db, &mut tx_rw)?;

        // generate a `ChainId` for the popped blocks.
        let old_main_chain_id = ChainId(rand::random());

        // pop the blocks
        for _ in 0..numb_blocks {
            crate::ops::block::pop_block(db, Some(old_main_chain_id), &mut tx_rw, &mut tapes)?;
        }

        tx_rw.commit()?;
        tapes.flush(Flush::NoSync)?;
        Ok(BlockchainResponse::PopBlocks(old_main_chain_id))
    };

    loop {
        let result = result();

        if matches!(
            result,
            Err(BlockchainError::Heed(heed::Error::Mdb(MdbError::MapFull)))
        ) {
            todo!();
            continue;
        }

        return result;
    }
}

/// [`BlockchainWriteRequest::FlushAltBlocks`].
#[inline]
fn flush_alt_blocks(db: &Blockchain) -> ResponseResult {
    let result = || {
        let mut tx_rw = db.dynamic_tables.write_txn()?;

        crate::ops::alt_block::flush_alt_blocks(db, &mut tx_rw)?;

        tx_rw.commit()?;

        Ok(BlockchainResponse::Ok)
    };

    loop {
        let result = result();

        if matches!(
            result,
            Err(BlockchainError::Heed(heed::Error::Mdb(MdbError::MapFull)))
        ) {
            todo!();
            continue;
        }

        return result;
    }
}
