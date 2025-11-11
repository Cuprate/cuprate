//! Database writer thread definitions and logic.

use std::net::Shutdown::Read;
//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use cuprate_database::{ConcreteEnv, DbResult, Env, EnvInner, RuntimeError, TxRw};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_linear_tapes::Flush;
use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, ChainId, VerifiedBlockInformation,
};

use crate::{
    service::types::{BlockchainWriteHandle, ResponseResult},
    tables::OpenTables,
    Database,
};
use crate::database::TX_INFOS;
use crate::ops::block::add_blocks_to_tapes;
use crate::types::TxInfo;

/// Write functions within this module abort if the write transaction
/// could not be aborted successfully to maintain atomicity.
///
/// This is the panic message if the `abort()` fails.
const TX_RW_ABORT_FAIL: &str =
    "Could not maintain blockchain database atomicity by aborting write transaction";

//---------------------------------------------------------------------------------------------------- init_write_service
/// Initialize the blockchain write service from a [`ConcreteEnv`].
pub fn init_write_service(env: Arc<Database>) -> BlockchainWriteHandle {
    DatabaseWriteHandle::init(env, handle_blockchain_request)
}

//---------------------------------------------------------------------------------------------------- handle_bc_request
/// Handle an incoming [`BlockchainWriteRequest`], returning a [`BlockchainResponse`].
fn handle_blockchain_request(
    env: &Arc<Database>,
    req: &BlockchainWriteRequest,
) -> DbResult<BlockchainResponse> {
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
fn write_block(db: &Database, block: &VerifiedBlockInformation) -> ResponseResult {
    write_blocks(db, std::slice::from_ref(block))
}

/// [`BlockchainWriteRequest::BatchWriteBlocks`].
#[inline]
fn write_blocks(db: &Database, blocks: &[VerifiedBlockInformation]) -> ResponseResult {
    let mut tapes = db.linear_tapes.appender();
    let mut numb_transactions = tapes.fixed_sized_tape_appender::<TxInfo>(TX_INFOS).len();
    add_blocks_to_tapes(blocks, &mut tapes)?;

    let mut tapes = Some(tapes);

    let mut result = move || {
        let env_inner = db.dynamic_tables.env_inner();
        let tx_rw = env_inner.tx_rw()?;

        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        for block in blocks {
           crate::ops::block::add_block_to_dynamic_tables(block, &mut numb_transactions, &mut tables_mut)?;
        }

        drop(tables_mut);

        if let Some(tapes) = tapes.take() {
            tapes.flush(Flush::NoSync)?;
        }

        TxRw::commit(tx_rw)?;

        Ok(BlockchainResponse::Ok)
    };

    loop{
        let result = result();

        if matches!(result, Err(RuntimeError::ResizeNeeded)) {
            db.dynamic_tables.resize_map(None);
            continue;
        }

        return result
    }
}

/// [`BlockchainWriteRequest::WriteAltBlock`].
#[inline]
fn write_alt_block(db: &Database, block: &AltBlockInformation) -> ResponseResult {
    let result = || {
        let env_inner = db.dynamic_tables.env_inner();
        let tx_rw = env_inner.tx_rw()?;

        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        crate::ops::alt_block::add_alt_block(block, &mut tables_mut)?;

        drop(tables_mut);
        TxRw::commit(tx_rw)?;

        Ok(BlockchainResponse::Ok)
    };

    loop{
        let result = result();

        if matches!(result, Err(RuntimeError::ResizeNeeded)) {
            db.dynamic_tables.resize_map(None);
            continue;
        }

        return result
    }
}

/// [`BlockchainWriteRequest::PopBlocks`].
fn pop_blocks(db: &Database, numb_blocks: usize) -> ResponseResult {
    // FIXME: turn this function into a try block once stable.
    let mut result = || {
        let env_inner = db.dynamic_tables.env_inner();
        let mut tx_rw = env_inner.tx_rw()?;
        let mut tapes = db.linear_tapes.popper();

        // flush all the current alt blocks as they may reference blocks to be popped.
        crate::ops::alt_block::flush_alt_blocks(&env_inner, &mut tx_rw)?;

        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        // generate a `ChainId` for the popped blocks.
        let old_main_chain_id = ChainId(rand::random());

        // pop the blocks
        for _ in 0..numb_blocks {
            crate::ops::block::pop_block(Some(old_main_chain_id), &mut tables_mut, &mut tapes)?;
        }

        drop(tables_mut);

        TxRw::commit(tx_rw)?;
        tapes.flush(Flush::NoSync)?;
        Ok(BlockchainResponse::PopBlocks(old_main_chain_id))

    };

    loop{
        let result = result();

        if matches!(result, Err(RuntimeError::ResizeNeeded)) {
            db.dynamic_tables.resize_map(None);
            continue;
        }

        return result
    }
}

/// [`BlockchainWriteRequest::FlushAltBlocks`].
#[inline]
fn flush_alt_blocks(db: &Database) -> ResponseResult {
    let result = || {
        let env_inner = db.dynamic_tables.env_inner();
        let mut tx_rw = env_inner.tx_rw()?;

        crate::ops::alt_block::flush_alt_blocks(&env_inner, &mut tx_rw)?;

        TxRw::commit(tx_rw)?;

        Ok(BlockchainResponse::Ok)

    };

    loop{
        let result = result();

        if matches!(result, Err(RuntimeError::ResizeNeeded)) {
            db.dynamic_tables.resize_map(None);
            continue;
        }

        return result
    }
}
