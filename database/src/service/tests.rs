//! `crate::service` tests.
//!
//! This module contains general tests for the `service` implementation.
//!
//! Testing a thread-pool is slightly more complicated,
//! so this file provides TODO.

// This is only imported on `#[cfg(test)]` in `mod.rs`.

#![allow(
    clippy::significant_drop_tightening,
    clippy::await_holding_lock,
    clippy::too_many_lines
)]

//---------------------------------------------------------------------------------------------------- Use
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use pretty_assertions::assert_eq;
use tower::{Service, ServiceExt};

use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx2, block_v9_tx3};
use cuprate_types::{
    service::{ReadRequest, Response, WriteRequest},
    ExtendedBlockHeader, VerifiedBlockInformation,
};

use crate::{
    config::Config,
    ops::{
        block::{get_block_extended_header_from_height, get_block_info},
        blockchain::top_block_height,
        output::get_output,
    },
    service::{init, DatabaseReadHandle, DatabaseWriteHandle},
    tables::{KeyImages, Tables, TablesIter},
    tests::AssertTableLen,
    types::{Amount, AmountIndex, KeyImage, PreRctOutputId},
    ConcreteEnv, DatabaseIter, DatabaseRo, Env, EnvInner, RuntimeError,
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

/// This is the template used in the actual test functions below.
///
/// - Send write request(s)
/// - Receive response(s)
/// - Assert proper tables were mutated
/// - Assert read requests lead to expected responses
#[allow(clippy::future_not_send)] // INVARIANT: tests are using a single threaded runtime
async fn test_template(
    // Which block(s) to add?
    block_fns: &[fn() -> &'static VerifiedBlockInformation],
    // Total amount of generated coins after the block(s) have been added.
    cumulative_generated_coins: u64,
    // What are the table lengths be after the block(s) have been added?
    assert_table_len: AssertTableLen,
) {
    //----------------------------------------------------------------------- Write requests
    let (reader, mut writer, env, _tempdir) = init_service();

    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();

    // HACK: `add_block()` asserts blocks with non-sequential heights
    // cannot be added, to get around this, manually edit the block height.
    for (i, block_fn) in block_fns.iter().enumerate() {
        let mut block = block_fn().clone();
        block.height = i as u64;

        // Request a block to be written, assert it was written.
        let request = WriteRequest::WriteBlock(block);
        let response_channel = writer.call(request);
        let response = response_channel.await.unwrap();
        assert_eq!(response, Response::WriteBlockOk);
    }

    //----------------------------------------------------------------------- Reset the transaction
    drop(tables);
    drop(tx_ro);
    let tx_ro = env_inner.tx_ro().unwrap();
    let tables = env_inner.open_tables(&tx_ro).unwrap();

    //----------------------------------------------------------------------- Assert all table lengths are correct
    assert_table_len.assert(&tables);

    //----------------------------------------------------------------------- Read request prep
    // Next few lines are just for preparing the expected responses,
    // see further below for usage.

    let extended_block_header_0 = Ok(Response::BlockExtendedHeader(
        get_block_extended_header_from_height(&0, &tables).unwrap(),
    ));

    let extended_block_header_1 = if block_fns.len() > 1 {
        Ok(Response::BlockExtendedHeader(
            get_block_extended_header_from_height(&1, &tables).unwrap(),
        ))
    } else {
        Err(RuntimeError::KeyNotFound)
    };

    let block_hash_0 = Ok(Response::BlockHash(
        get_block_info(&0, tables.block_infos()).unwrap().block_hash,
    ));

    let block_hash_1 = if block_fns.len() > 1 {
        Ok(Response::BlockHash(
            get_block_info(&1, tables.block_infos()).unwrap().block_hash,
        ))
    } else {
        Err(RuntimeError::KeyNotFound)
    };

    let range_0_1 = Ok(Response::BlockExtendedHeaderInRange(vec![
        get_block_extended_header_from_height(&0, &tables).unwrap(),
    ]));

    let range_0_2 = if block_fns.len() >= 2 {
        Ok(Response::BlockExtendedHeaderInRange(vec![
            get_block_extended_header_from_height(&0, &tables).unwrap(),
            get_block_extended_header_from_height(&1, &tables).unwrap(),
        ]))
    } else {
        Err(RuntimeError::KeyNotFound)
    };

    let chain_height = {
        let height = top_block_height(tables.block_heights()).unwrap();
        let block_info = get_block_info(&height, tables.block_infos()).unwrap();
        Ok(Response::ChainHeight(height, block_info.block_hash))
    };

    let cumulative_generated_coins = Ok(Response::GeneratedCoins(cumulative_generated_coins));

    let num_req = tables
        .outputs_iter()
        .keys()
        .unwrap()
        .map(Result::unwrap)
        .map(|key| key.amount)
        .collect::<Vec<Amount>>();

    let num_resp = Ok(Response::NumberOutputsWithAmount(
        num_req
            .iter()
            .map(|amount| match tables.num_outputs().get(amount) {
                // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
                #[allow(clippy::cast_possible_truncation)]
                Ok(count) => (*amount, count as usize),
                Err(RuntimeError::KeyNotFound) => (*amount, 0),
                Err(e) => panic!(),
            })
            .collect::<HashMap<Amount, usize>>(),
    ));

    // Contains a fake non-spent key-image.
    let ki_req = HashSet::from([[0; 32]]);
    let ki_resp = Ok(Response::CheckKIsNotSpent(true));

    //----------------------------------------------------------------------- Assert expected response
    // Assert read requests lead to the expected responses.
    for (request, expected_response) in [
        (ReadRequest::BlockExtendedHeader(0), extended_block_header_0),
        (ReadRequest::BlockExtendedHeader(1), extended_block_header_1),
        (ReadRequest::BlockHash(0), block_hash_0),
        (ReadRequest::BlockHash(1), block_hash_1),
        (ReadRequest::BlockExtendedHeaderInRange(0..1), range_0_1),
        (ReadRequest::BlockExtendedHeaderInRange(0..2), range_0_2),
        (ReadRequest::ChainHeight, chain_height),
        (ReadRequest::GeneratedCoins, cumulative_generated_coins),
        (ReadRequest::NumberOutputsWithAmount(num_req), num_resp),
        (ReadRequest::CheckKIsNotSpent(ki_req), ki_resp),
    ] {
        let response = reader.clone().oneshot(request).await;
        println!("response: {response:#?}, expected_response: {expected_response:#?}");
        match response {
            Ok(resp) => assert_eq!(resp, expected_response.unwrap()),
            Err(ref e) => assert!(matches!(response, expected_response)),
        }
    }

    //----------------------------------------------------------------------- Key image checks
    // Assert each key image we inserted comes back as "spent".
    for key_image in tables.key_images_iter().keys().unwrap() {
        let key_image = key_image.unwrap();
        let request = ReadRequest::CheckKIsNotSpent(HashSet::from([key_image]));
        let response = reader.clone().oneshot(request).await;
        println!("response: {response:#?}, key_image: {key_image:#?}");
        assert_eq!(response.unwrap(), Response::CheckKIsNotSpent(false));
    }

    //----------------------------------------------------------------------- Output checks
    // FIXME: Constructing the correct `OutputOnChain` here is
    // hard to do without the code inside `service/read.rs`.
    // For now, we're only testing the amount of outputs returned
    // is as expected, but not if the output values themselves are correct.

    // Create the map of amounts and amount indices.
    //
    // FIXME: There's definitely a better way to map
    // `Vec<PreRctOutputId>` -> `HashMap<u64, HashSet<u64>>`
    let (map, output_count) = {
        let ids = tables
            .outputs_iter()
            .keys()
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<PreRctOutputId>>();

        // Used later to compare the amount of Outputs
        // returned in the Response is equal to the amount
        // we asked for.
        let output_count = ids.len();

        let mut map = HashMap::<Amount, HashSet<AmountIndex>>::new();
        for id in ids {
            map.entry(id.amount)
                .and_modify(|set| {
                    set.insert(id.amount_index);
                })
                .or_insert_with(|| HashSet::from([id.amount_index]));
        }

        (map, output_count)
    };

    // Send a request for every output we inserted before.
    let request = ReadRequest::Outputs(map.clone());
    let response = reader.clone().oneshot(request).await;
    println!("Response::Outputs response: {response:#?}");
    let Ok(Response::Outputs(response)) = response else {
        panic!()
    };

    // Assert amount of `Amount`'s are the same.
    assert_eq!(map.len(), response.len());

    // Assert we get back the same map of
    // `Amount`'s and `AmountIndex`'s.
    let mut response_output_count = 0;
    for (amount, output_map) in response {
        let amount_index_set = map.get(&amount).unwrap();

        for (amount_index, output) in output_map {
            response_output_count += 1;
            assert!(amount_index_set.contains(&amount_index));
            // FIXME: assert output correctness.
        }
    }

    // Assert the amount of `Output`'s returned is as expected.
    let table_output_len = tables.outputs().len().unwrap();
    assert_eq!(output_count as u64, table_output_len);
    assert_eq!(output_count, response_output_count);
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
    test_template(
        &[block_v1_tx2],
        14_535_350_982_449,
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
        },
    )
    .await;
}

/// Assert write/read correctness of [`block_v9_tx3`].
#[tokio::test]
async fn v9_tx3() {
    test_template(
        &[block_v9_tx3],
        3_403_774_022_163,
        AssertTableLen {
            block_infos: 1,
            block_blobs: 1,
            block_heights: 1,
            key_images: 4,
            num_outputs: 0,
            pruned_tx_blobs: 0,
            prunable_hashes: 0,
            outputs: 0,
            prunable_tx_blobs: 0,
            rct_outputs: 6,
            tx_blobs: 3,
            tx_ids: 3,
            tx_heights: 3,
            tx_unlock_time: 0,
        },
    )
    .await;
}

/// Assert write/read correctness of [`block_v16_tx0`].
#[tokio::test]
async fn v16_tx0() {
    test_template(
        &[block_v16_tx0],
        600_000_000_000,
        AssertTableLen {
            block_infos: 1,
            block_blobs: 1,
            block_heights: 1,
            key_images: 0,
            num_outputs: 0,
            pruned_tx_blobs: 0,
            prunable_hashes: 0,
            outputs: 0,
            prunable_tx_blobs: 0,
            rct_outputs: 0,
            tx_blobs: 0,
            tx_ids: 0,
            tx_heights: 0,
            tx_unlock_time: 0,
        },
    )
    .await;
}
