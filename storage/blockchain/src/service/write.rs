//! Database writer thread definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use cuprate_database::{ConcreteEnv, Env, EnvInner, RuntimeError, TxRw};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    VerifiedBlockInformation,
};

use crate::{
    ops,
    service::types::{BlockchainWriteHandle, ResponseResult},
    tables::OpenTables,
};

//---------------------------------------------------------------------------------------------------- init_write_service
/// Initialize the blockchain write service from a [`ConcreteEnv`].
pub fn init_write_service(env: Arc<ConcreteEnv>) -> BlockchainWriteHandle {
    DatabaseWriteHandle::init(env, handle_blockchain_request)
}

//---------------------------------------------------------------------------------------------------- handle_bc_request
/// Handle an incoming [`BlockchainWriteRequest`], returning a [`BlockchainResponse`].
fn handle_blockchain_request(
    env: &ConcreteEnv,
    req: &BlockchainWriteRequest,
) -> Result<BlockchainResponse, RuntimeError> {
    match req {
        BlockchainWriteRequest::WriteBlock(block) => write_block(env, block),
        BlockchainWriteRequest::PopBlocks(nblocks) => pop_blocks(env, *nblocks),
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
        ops::block::add_block(block, &mut tables_mut)
    };

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::WriteBlock)
        }
        Err(e) => {
            // INVARIANT: ensure database atomicity by aborting
            // the transaction on `add_block()` failures.
            TxRw::abort(tx_rw)
                .expect("could not maintain database atomicity by aborting write transaction");
            Err(e)
        }
    }
}

/// [`BlockchainWriteRequest::PopBlocks`].
#[inline]
fn pop_blocks(env: &ConcreteEnv, nblocks: u64) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let result = || {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        let mut height = 0;

        for _ in 0..nblocks {
            (height, _, _) = ops::block::pop_block(&mut tables_mut)?;
        }

        Ok(height)
    };

    match result() {
        Ok(height) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::PopBlocks(height))
        }
        Err(e) => {
            // INVARIANT: ensure database atomicity by aborting
            // the transaction on `add_block()` failures.
            TxRw::abort(tx_rw)
                .expect("could not maintain database atomicity by aborting write transaction");
            Err(e)
        }
    }
}
