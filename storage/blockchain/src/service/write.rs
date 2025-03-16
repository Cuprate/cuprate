//! Database writer thread definitions and logic.

use std::collections::HashMap;
//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;
use curve25519_dalek::edwards::CompressedEdwardsY;
use cuprate_database::{ConcreteEnv, DatabaseRo, DbResult, Env, EnvInner, TxRw};
use cuprate_database_service::DatabaseWriteHandle;
use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, Chain, ChainId, VerifiedBlockInformation,
};

use crate::{
    service::{
        free::map_valid_alt_block_to_verified_block,
        types::{BlockchainWriteHandle, ResponseResult},
    },
    tables::{OpenTables, Tables},
    types::AltBlockHeight,
};

/// Write functions within this module abort if the write transaction
/// could not be aborted successfully to maintain atomicity.
///
/// This is the panic message if the `abort()` fails.
const TX_RW_ABORT_FAIL: &str =
    "Could not maintain blockchain database atomicity by aborting write transaction";

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
) -> DbResult<BlockchainResponse> {
    match req {
        BlockchainWriteRequest::WriteBlock(block) => write_block(env, block),
        BlockchainWriteRequest::BatchWriteBlocks{blocks, miner_commitments} => write_blocks(env, blocks, miner_commitments),
        BlockchainWriteRequest::WriteAltBlock(alt_block) => write_alt_block(env, alt_block),
        BlockchainWriteRequest::PopBlocks(numb_blocks) => pop_blocks(env, *numb_blocks),
        BlockchainWriteRequest::ReverseReorg(old_main_chain_id) => {
            reverse_reorg(env, *old_main_chain_id)
        }
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
        crate::ops::block::add_block(block, &mut tables_mut, None)
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
fn write_blocks(env: &ConcreteEnv, block: &Vec<VerifiedBlockInformation>, miner_commitments: &HashMap<u64, CompressedEdwardsY>) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let result = {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        for block in block {
            crate::ops::block::add_block(block, &mut tables_mut, Some(miner_commitments))?;
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

/// [`BlockchainWriteRequest::ReverseReorg`].
fn reverse_reorg(env: &ConcreteEnv, chain_id: ChainId) -> ResponseResult {
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw()?;

    // FIXME: turn this function into a try block once stable.
    let mut result = || {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;

        let chain_info = tables_mut.alt_chain_infos().get(&chain_id.into())?;
        // Although this doesn't guarantee the chain was popped from the main-chain, it's an easy
        // thing for us to check.
        assert_eq!(Chain::from(chain_info.parent_chain), Chain::Main);

        let top_block_height =
            crate::ops::blockchain::top_block_height(tables_mut.block_heights())?;

        // pop any blocks that were added as part of a re-org.
        for _ in chain_info.common_ancestor_height..top_block_height {
            crate::ops::block::pop_block(None, &mut tables_mut)?;
        }

        // Add the old main chain blocks back to the main chain.
        for height in (chain_info.common_ancestor_height + 1)..chain_info.chain_height {
            let alt_block = crate::ops::alt_block::get_alt_block(
                &AltBlockHeight {
                    chain_id: chain_id.into(),
                    height,
                },
                &tables_mut,
            )?;
            let verified_block = map_valid_alt_block_to_verified_block(alt_block);
            crate::ops::block::add_block(&verified_block, &mut tables_mut, None)?;
        }

        drop(tables_mut);
        crate::ops::alt_block::flush_alt_blocks(&env_inner, &mut tx_rw)?;

        Ok(())
    };

    match result() {
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
