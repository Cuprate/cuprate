//! Database writer thread definitions and logic.
//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use cuprate_database::{ConcreteEnv, DbResult, Env, EnvInner, TxRw};
use cuprate_linear_tape::Flush;
use cuprate_types::{
    blockchain::{BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, ChainId, VerifiedBlockInformation,
};

use crate::{service::types::ResponseResult, tables::OpenTables, BlockchainDatabase};
use crate::ops::block::{add_prunable_blocks_blobs, add_pruned_blocks_blobs};

/// Write functions within this module abort if the write transaction
/// could not be aborted successfully to maintain atomicity.
///
/// This is the panic message if the `abort()` fails.
const TX_RW_ABORT_FAIL: &str =
    "Could not maintain blockchain database atomicity by aborting write transaction";

//---------------------------------------------------------------------------------------------------- handle_bc_request
/// Handle an incoming [`BlockchainWriteRequest`], returning a [`BlockchainResponse`].
pub fn map_write_request<E: Env>(
    env: &BlockchainDatabase<E>,
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
fn write_block<E: Env>(
    env: &BlockchainDatabase<E>,
    block: &VerifiedBlockInformation,
) -> ResponseResult {
    write_blocks(env, std::slice::from_ref(block))
}

/// [`BlockchainWriteRequest::BatchWriteBlocks`].
#[inline]
fn write_blocks<E: Env>(
    env: &BlockchainDatabase<E>,
    block: &[VerifiedBlockInformation],
) -> ResponseResult {
    let env_inner = env.dynamic_tables.env_inner();
    let tx_rw = env_inner.tx_rw()?;
    let mut tapes = env.tapes.upgradable_read();

    let result = {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;

        let mut pruned_blob_idx = add_pruned_blocks_blobs(block, &mut tapes)?;

        let start_height = block[0].height;
        let first_block_pruning_seed = cuprate_pruning::DecompressedPruningSeed::new(cuprate_pruning::get_block_pruning_stripe(start_height, usize::MAX, 3).unwrap(), 3).unwrap();
        let next_stripe_height = first_block_pruning_seed.get_next_unpruned_block(start_height, 500_000_000).unwrap();

        let (first_stripe, next_stripe) = block.split_at(next_stripe_height - start_height);

        let mut numb_rct_outs = tapes.rct_outputs.reader().unwrap().len() as u64;
        let mut rct_outputs = Vec::with_capacity(10_000);

        for blocks in [first_stripe, next_stripe] {
            if blocks.is_empty() {
                continue;
            }
           let mut prunable_idx = add_prunable_blocks_blobs(blocks, &mut tapes)?;
            for block in blocks {
                crate::ops::block::add_block(block, &mut pruned_blob_idx, &mut prunable_idx, &mut numb_rct_outs, &mut rct_outputs, &mut tables_mut)?;
            }
        }

        let mut appender = unsafe { tapes.rct_outputs.appender().unwrap() };

        if appender.push_entries(&rct_outputs).is_err() {
            tapes.with_upgraded(|tapes| {
                let file_size = tapes.rct_outputs.map_size() as u64;
                tapes.rct_outputs.resize(file_size + 1 * 1024 * 1024 * 1024)?;

                let mut appender = unsafe { tapes.rct_outputs.appender().unwrap() };
                appender.push_entries(&rct_outputs).unwrap();
                appender.flush(Flush::Async)
            })?;
        } else {
            appender.flush(Flush::Async)?;
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
fn write_alt_block<E: Env>(
    env: &BlockchainDatabase<E>,
    block: &AltBlockInformation,
) -> ResponseResult {
    let env_inner = env.dynamic_tables.env_inner();
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
fn pop_blocks<E: Env>(env: &BlockchainDatabase<E>, numb_blocks: usize) -> ResponseResult {
    let env_inner = env.dynamic_tables.env_inner();
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
            crate::ops::block::pop_block(
                Some(old_main_chain_id),
                &mut tables_mut,
                &mut env.tapes.write(),
            )?;
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
fn flush_alt_blocks<E: Env>(env: &BlockchainDatabase<E>) -> ResponseResult {
    let env_inner = env.dynamic_tables.env_inner();
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
