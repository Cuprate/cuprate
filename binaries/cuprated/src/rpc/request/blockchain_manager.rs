//! Functions for TODO: doc enum message.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{anyhow, Error};
use futures::StreamExt;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    Chain, ExtendedBlockHeader, HardFork, OutputOnChain, VerifiedBlockInformation,
};

use crate::rpc::{CupratedRpcHandler, CupratedRpcHandlerState};

/// TODO: doc enum message
pub(super) async fn pop_blocks() -> Result<(u64, [u8; 32]), Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn prune() -> Result<(), Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn pruned() -> Result<bool, Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn relay_block() -> Result<bool, Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn syncing() -> Result<bool, Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn synced() -> Result<bool, Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn target() -> Result<bool, Error> {
    Ok(todo!())
}

/// TODO: doc enum message
pub(super) async fn target_height() -> Result<bool, Error> {
    Ok(todo!())
}
