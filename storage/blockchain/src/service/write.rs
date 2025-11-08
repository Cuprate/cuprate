//! Database writer thread definitions and logic.
//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use cuprate_database::{ConcreteEnv, DbResult, Env, EnvInner, TxRw};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, ChainId, VerifiedBlockInformation,
};

use crate::{service::types::{BlockchainWriteHandle, ResponseResult}, tables::OpenTables, Database};

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
fn write_block(env: &ConcreteEnv, block: &VerifiedBlockInformation) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let result = {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        crate::ops::block::add_block(block, &mut tables_mut)
    };

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::Ok)
        }
        Err(e) => {
            TxRw::abort(tx_rw).expect(TX_RW_ABORT_FAIL);
            Err(e)
        }
    }
}

/// [`BlockchainWriteRequest::BatchWriteBlocks`].
#[inline]
fn write_blocks(env: &ConcreteEnv, block: &Vec<VerifiedBlockInformation>) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let result = {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        for block in block {
            crate::ops::block::add_block(block, &mut tables_mut)?;
        }

        Ok(())
    };

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::Ok)
        }
        Err(e) => {
            TxRw::abort(tx_rw).expect(TX_RW_ABORT_FAIL);
            Err(e)
        }
    }
}

/// [`BlockchainWriteRequest::WriteAltBlock`].
#[inline]
fn write_alt_block(env: &ConcreteEnv, block: &AltBlockInformation) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let result = {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        crate::ops::alt_block::add_alt_block(block, &mut tables_mut)
    };

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::Ok)
        }
        Err(e) => {
            TxRw::abort(tx_rw).expect(TX_RW_ABORT_FAIL);
            Err(e)
        }
    }
}

/// [`BlockchainWriteRequest::PopBlocks`].
fn pop_blocks(env: &ConcreteEnv, numb_blocks: usize) -> ResponseResult {
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw()?;

    // FIXME: turn this function into a try block once stable.
    let mut result = || {
        // flush all the current alt blocks as they may reference blocks to be popped.
        crate::ops::alt_block::flush_alt_blocks(&env_inner, &mut tx_rw)?;

        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        // generate a `ChainId` for the popped blocks.
        let old_main_chain_id = ChainId(rand::random());

        // pop the blocks
        for _ in 0..numb_blocks {
            crate::ops::block::pop_block(Some(old_main_chain_id), &mut tables_mut)?;
        }

        Ok(old_main_chain_id)
    };

    match result() {
        Ok(old_main_chain_id) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::PopBlocks(old_main_chain_id))
        }
        Err(e) => {
            TxRw::abort(tx_rw).expect(TX_RW_ABORT_FAIL);
            Err(e)
        }
    }
}

/// [`BlockchainWriteRequest::FlushAltBlocks`].
#[inline]
fn flush_alt_blocks(env: &ConcreteEnv) -> ResponseResult {
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw()?;

    let result = crate::ops::alt_block::flush_alt_blocks(&env_inner, &mut tx_rw);

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::Ok)
        }
        Err(e) => {
            TxRw::abort(tx_rw).expect(TX_RW_ABORT_FAIL);
            Err(e)
        }
    }
}
