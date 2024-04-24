//! `crate::service` tests.
//!
//! This module contains general tests for the `service` implementation.
//!
//! Testing a thread-pool is slightly more complicated,
//! so this file provides TODO.

// This is only imported on `#[cfg(test)]` in `mod.rs`.

#![allow(unused_mut, clippy::significant_drop_tightening)]

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

//---------------------------------------------------------------------------------------------------- Use
use tower::{Service, ServiceExt};

use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx2, block_v9_tx3};
use cuprate_types::{
    service::{ReadRequest, Response, WriteRequest},
    ExtendedBlockHeader, VerifiedBlockInformation,
};

use crate::{
    config::Config,
    ops::block::{get_block_extended_header_from_height, get_block_info},
    service::{init, DatabaseReadHandle, DatabaseWriteHandle},
    tables::Tables,
    tests::AssertTableLen,
    ConcreteEnv, DatabaseRo, Env, EnvInner, RuntimeError,
};

//---------------------------------------------------------------------------------------------------- Helper functions
/// Initialize the `service`.
fn init_service() -> (
    DatabaseReadHandle,
    DatabaseWriteHandle,
    Arc<ConcreteEnv>,
    tempfile::TempDir,
) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let (reader, writer) = init(config).unwrap();
    let env = reader.env().clone();
    (reader, writer, env, tempdir)
}

/// Send a write request, and receive a response,
/// asserting the response the expected value.
async fn write_request(
    writer: &mut DatabaseWriteHandle,
    block_fn: fn() -> &'static VerifiedBlockInformation,
) {
    // HACK: `add_block()` asserts blocks with non-sequential heights
    // cannot be added, to get around this, manually edit the block height.
    let mut block = block_fn().clone();
    block.height = 0;

    // Request a block to be written, assert it was written.
    let request = WriteRequest::WriteBlock(block);
    let response_channel = writer.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::WriteBlockOk);
}

//---------------------------------------------------------------------------------------------------- Tests
/// Simply `init()` the service and then drop it.
///
/// If this test fails, something is very wrong.
#[test]
fn init_drop() {
    let (reader, writer, env, _tempdir) = init_service();
}

/// Assert write/read correctness of [`block_v1_tx2`].
#[tokio::test]
async fn v1_tx2() {
    let (reader, mut writer, env, _tempdir) = init_service();

    write_request(&mut writer, block_v1_tx2).await;

    // Assert the actual database tables were correctly modified.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();

    AssertTableLen {
        block_infos: 1,
        block_blobs: 1,
        block_heights: 1,
        key_images: 65,
        num_outputs: 38,
        pruned_tx_blobs: 0,
        prunable_hashes: 0,
        outputs: 107,
        prunable_tx_blobs: 0,
        rct_outputs: 0,
        tx_blobs: 2,
        tx_ids: 2,
        tx_heights: 2,
        tx_unlock_time: 0,
    }
    .assert(&tables);

    let height = 0;
    let extended_block_header = get_block_extended_header_from_height(&height, &tables).unwrap();
    let block_info = get_block_info(&height, tables.block_infos()).unwrap();

    // Assert reads are correct.
    for (request, expected_response) in [
        // Each tuple is a `Request` + `Result<Response, RuntimeError>` pair.
        (
            ReadRequest::BlockExtendedHeader(0), // The request to send to the service
            Ok(Response::BlockExtendedHeader(extended_block_header)), // The expected response
        ),
        (
            ReadRequest::BlockExtendedHeader(1),
            Err(RuntimeError::KeyNotFound),
        ),
        (
            ReadRequest::BlockHash(0),
            Ok(Response::BlockHash(block_info.block_hash)),
        ),
        (ReadRequest::BlockHash(1), Err(RuntimeError::KeyNotFound)),
        (
            ReadRequest::BlockExtendedHeaderInRange(0..1),
            Ok(Response::BlockExtendedHeaderInRange(vec![
                extended_block_header,
            ])),
        ),
        (
            ReadRequest::BlockExtendedHeaderInRange(0..2),
            Err(RuntimeError::KeyNotFound),
        ),
        (
            ReadRequest::ChainHeight,
            Ok(Response::ChainHeight(height, block_info.block_hash)),
        ),
        (ReadRequest::GeneratedCoins, Ok(Response::GeneratedCoins(0))),
        // (ReadRequest::Outputs(HashMap<u64, HashSet<u64>>), ),
        // (ReadRequest::NumberOutputsWithAmount(Vec<u64>), ),
        // (ReadRequest::CheckKIsNotSpent(HashSet<[u8; 32]>), ),
    ] {
        let response_channel = reader.clone().oneshot(request);
        let response = response_channel.await;
        println!("response: {response:#?}, expected_response: {expected_response:#?}");
        assert!(matches!(response, expected_response));
    }
}

/// Assert write/read correctness of [`block_v9_tx3`].
#[tokio::test]
async fn v9_tx3() {
    let (reader, mut writer, env, _tempdir) = init_service();

    write_request(&mut writer, block_v9_tx3).await;

    // Assert the actual database tables were correctly modified.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();

    assert_eq!(tables.block_infos().len().unwrap(), 1);
    assert_eq!(tables.block_blobs().len().unwrap(), 1);
    assert_eq!(tables.block_heights().len().unwrap(), 1);
    assert_eq!(tables.key_images().len().unwrap(), 4);
    assert_eq!(tables.num_outputs().len().unwrap(), 0);
    assert_eq!(tables.pruned_tx_blobs().len().unwrap(), 0);
    assert_eq!(tables.prunable_hashes().len().unwrap(), 0);
    assert_eq!(tables.outputs().len().unwrap(), 0);
    assert_eq!(tables.prunable_tx_blobs().len().unwrap(), 0);
    assert_eq!(tables.rct_outputs().len().unwrap(), 6);
    assert_eq!(tables.tx_blobs().len().unwrap(), 3);
    assert_eq!(tables.tx_ids().len().unwrap(), 3);
    assert_eq!(tables.tx_heights().len().unwrap(), 3);
    assert_eq!(tables.tx_unlock_time().len().unwrap(), 0);
}

/// Assert write/read correctness of [`block_v16_tx0`].
#[tokio::test]
async fn v16_tx0() {
    let (reader, mut writer, env, _tempdir) = init_service();

    write_request(&mut writer, block_v16_tx0).await;

    // Assert the actual database tables were correctly modified.
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();

    assert_eq!(tables.block_infos().len().unwrap(), 1);
    assert_eq!(tables.block_blobs().len().unwrap(), 1);
    assert_eq!(tables.block_heights().len().unwrap(), 1);
    assert_eq!(tables.key_images().len().unwrap(), 0);
    assert_eq!(tables.num_outputs().len().unwrap(), 0);
    assert_eq!(tables.pruned_tx_blobs().len().unwrap(), 0);
    assert_eq!(tables.prunable_hashes().len().unwrap(), 0);
    assert_eq!(tables.outputs().len().unwrap(), 0);
    assert_eq!(tables.prunable_tx_blobs().len().unwrap(), 0);
    assert_eq!(tables.rct_outputs().len().unwrap(), 0);
    assert_eq!(tables.tx_blobs().len().unwrap(), 0);
    assert_eq!(tables.tx_ids().len().unwrap(), 0);
    assert_eq!(tables.tx_heights().len().unwrap(), 0);
    assert_eq!(tables.tx_unlock_time().len().unwrap(), 0);
}

// /// Send a read request, and receive a response,
// /// asserting the response the expected value.
// #[tokio::test]
// async fn read_request() {
//     let (reader, writer, _tempdir) = init_service();

//     for (request, expected_response) in [
//         (ReadRequest::Example1, Response::Example1),
//         (ReadRequest::Example2(123), Response::Example2(123)),
//         (
//             ReadRequest::Example3("hello".into()),
//             Response::Example3("hello".into()),
//         ),
//     ] {
//         // This calls `poll_ready()` asserting we have a permit before `call()`.
//         let response_channel = reader.clone().oneshot(request);
//         let response = response_channel.await.unwrap();
//         assert_eq!(response, expected_response);
//     }
// }
