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

impl CupratedRpcHandlerState {}
