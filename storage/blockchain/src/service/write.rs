//! Database writer thread definitions and logic.
//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use cuprate_database::{ConcreteEnv, DatabaseRo, DatabaseRw, Env, EnvInner, RuntimeError, TxRw};
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
    tables::{OpenTables, Tables, TablesMut},
    types::{AltBlockHeight, AltChainInfo},
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
        crate::ops::block::add_block(block, &mut tables_mut)
    };

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::Ok)
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
            // INVARIANT: ensure database atomicity by aborting
            // the transaction on `add_block()` failures.
            TxRw::abort(tx_rw)
                .expect("could not maintain database atomicity by aborting write transaction");
            Err(e)
        }
    }
}

/// [`BlockchainWriteRequest::PopBlocks`].
fn pop_blocks(env: &ConcreteEnv, numb_blocks: usize) -> ResponseResult {
    let env_inner = env.env_inner();
    let mut tx_rw = env_inner.tx_rw()?;

    // TODO: turn this function into a try block once stable.
    let mut result = || {
        // flush all the current alt blocks as they may reference blocks to be popped.
        crate::ops::alt_block::flush_alt_blocks(&env_inner, &mut tx_rw)?;

        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        // generate a `ChainId` for the popped blocks.
        let old_main_chain_id = ChainId(rand::random());

        // pop the blocks
        let mut last_block_height = 0;
        for _ in 0..numb_blocks {
            (last_block_height, _, _) =
                crate::ops::block::pop_block(Some(old_main_chain_id), &mut tables_mut)?;
        }

        // Update the alt_chain_info with the correct information.
        tables_mut.alt_chain_infos_mut().put(
            &old_main_chain_id.into(),
            &AltChainInfo {
                parent_chain: Chain::Main.into(),
                common_ancestor_height: last_block_height - 1,
                chain_height: last_block_height + numb_blocks,
            },
        )?;

        Ok(old_main_chain_id)
    };

    match result() {
        Ok(old_main_chain_id) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::PopBlocks(old_main_chain_id))
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

/// [`BlockchainWriteRequest::ReverseReorg`].
fn reverse_reorg(env: &ConcreteEnv, chain_id: ChainId) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    // TODO: turn this function into a try block once stable.
    let result = || {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;

        let chain_info = tables_mut.alt_chain_infos().get(&chain_id.into())?;
        // Although this doesn't guarantee the chain was popped from the main-chain, it's an easy
        // thing for us to check.
        assert_eq!(Chain::from(chain_info.parent_chain), Chain::Main);

        let tob_block_height =
            crate::ops::blockchain::top_block_height(tables_mut.block_heights())?;

        // pop any blocks that were added as part of a re-org.
        for _ in chain_info.common_ancestor_height..tob_block_height {
            crate::ops::block::pop_block(None, &mut tables_mut)?;
        }

        // Rust borrow rules requires us to collect into a Vec first before looping over the Vec.
        let alt_blocks = (chain_info.common_ancestor_height..chain_info.chain_height)
            .map(|height| {
                crate::ops::alt_block::get_alt_block(
                    &AltBlockHeight {
                        chain_id: chain_id.into(),
                        height,
                    },
                    &tables_mut,
                )
            })
            .collect::<Vec<_>>();

        // Add the old main chain blocks back to the main chain.
        for res_alt_block in alt_blocks {
            let alt_block = res_alt_block?;

            let verified_block = map_valid_alt_block_to_verified_block(alt_block);

            crate::ops::block::add_block(&verified_block, &mut tables_mut)?;
        }

        Ok(())
    };

    match result() {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BlockchainResponse::Ok)
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
            // INVARIANT: ensure database atomicity by aborting
            // the transaction on `add_block()` failures.
            TxRw::abort(tx_rw)
                .expect("could not maintain database atomicity by aborting write transaction");
            Err(e)
        }
    }
}
