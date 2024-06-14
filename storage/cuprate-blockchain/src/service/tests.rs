//! `crate::service` tests.
//!
//! This module contains general tests for the `service` implementation.

// This is only imported on `#[cfg(test)]` in `mod.rs`.
#![allow(clippy::await_holding_lock, clippy::too_many_lines)]

//---------------------------------------------------------------------------------------------------- Use
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use pretty_assertions::assert_eq;
use tower::{Service, ServiceExt};

use cuprate_database::{ConcreteEnv, DatabaseIter, DatabaseRo, Env, EnvInner, RuntimeError};
use cuprate_test_utils::data::{block_v16_tx0, block_v1_tx2, block_v9_tx3};
use cuprate_types::{
    blockchain::{BCReadRequest, BCResponse, BCWriteRequest},
    OutputOnChain, VerifiedBlockInformation,
};

use crate::{
    config::ConfigBuilder,
    open_tables::OpenTables,
    ops::{
        block::{get_block_extended_header_from_height, get_block_info},
        blockchain::chain_height,
        output::id_to_output_on_chain,
    },
    service::{init, DatabaseReadHandle, DatabaseWriteHandle},
    tables::{Tables, TablesIter},
    tests::AssertTableLen,
    types::{Amount, AmountIndex, PreRctOutputId},
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
    let config = ConfigBuilder::new()
        .db_directory(Cow::Owned(tempdir.path().into()))
        .low_power()
        .build();
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
        let request = BCWriteRequest::WriteBlock(block);
        let response_channel = writer.call(request);
        let response = response_channel.await.unwrap();
        assert_eq!(response, BCResponse::WriteBlockOk);
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

    let extended_block_header_0 = Ok(BCResponse::BlockExtendedHeader(
        get_block_extended_header_from_height(&0, &tables).unwrap(),
    ));

    let extended_block_header_1 = if block_fns.len() > 1 {
        Ok(BCResponse::BlockExtendedHeader(
            get_block_extended_header_from_height(&1, &tables).unwrap(),
        ))
    } else {
        Err(RuntimeError::KeyNotFound)
    };

    let block_hash_0 = Ok(BCResponse::BlockHash(
        get_block_info(&0, tables.block_infos()).unwrap().block_hash,
    ));

    let block_hash_1 = if block_fns.len() > 1 {
        Ok(BCResponse::BlockHash(
            get_block_info(&1, tables.block_infos()).unwrap().block_hash,
        ))
    } else {
        Err(RuntimeError::KeyNotFound)
    };

    let range_0_1 = Ok(BCResponse::BlockExtendedHeaderInRange(vec![
        get_block_extended_header_from_height(&0, &tables).unwrap(),
    ]));

    let range_0_2 = if block_fns.len() >= 2 {
        Ok(BCResponse::BlockExtendedHeaderInRange(vec![
            get_block_extended_header_from_height(&0, &tables).unwrap(),
            get_block_extended_header_from_height(&1, &tables).unwrap(),
        ]))
    } else {
        Err(RuntimeError::KeyNotFound)
    };

    let chain_height = {
        let height = chain_height(tables.block_heights()).unwrap();
        let block_info = get_block_info(&height.saturating_sub(1), tables.block_infos()).unwrap();
        Ok(BCResponse::ChainHeight(height, block_info.block_hash))
    };

    let cumulative_generated_coins = Ok(BCResponse::GeneratedCoins(cumulative_generated_coins));

    let num_req = tables
        .outputs_iter()
        .keys()
        .unwrap()
        .map(Result::unwrap)
        .map(|key| key.amount)
        .collect::<Vec<Amount>>();

    let num_resp = Ok(BCResponse::NumberOutputsWithAmount(
        num_req
            .iter()
            .map(|amount| match tables.num_outputs().get(amount) {
                // INVARIANT: #[cfg] @ lib.rs asserts `usize == u64`
                #[allow(clippy::cast_possible_truncation)]
                Ok(count) => (*amount, count as usize),
                Err(RuntimeError::KeyNotFound) => (*amount, 0),
                Err(e) => panic!("{e:?}"),
            })
            .collect::<HashMap<Amount, usize>>(),
    ));

    // Contains a fake non-spent key-image.
    let ki_req = HashSet::from([[0; 32]]);
    let ki_resp = Ok(BCResponse::KeyImagesSpent(false));

    //----------------------------------------------------------------------- Assert expected response
    // Assert read requests lead to the expected responses.
    for (request, expected_response) in [
        (
            BCReadRequest::BlockExtendedHeader(0),
            extended_block_header_0,
        ),
        (
            BCReadRequest::BlockExtendedHeader(1),
            extended_block_header_1,
        ),
        (BCReadRequest::BlockHash(0), block_hash_0),
        (BCReadRequest::BlockHash(1), block_hash_1),
        (BCReadRequest::BlockExtendedHeaderInRange(0..1), range_0_1),
        (BCReadRequest::BlockExtendedHeaderInRange(0..2), range_0_2),
        (BCReadRequest::ChainHeight, chain_height),
        (BCReadRequest::GeneratedCoins, cumulative_generated_coins),
        (BCReadRequest::NumberOutputsWithAmount(num_req), num_resp),
        (BCReadRequest::KeyImagesSpent(ki_req), ki_resp),
    ] {
        let response = reader.clone().oneshot(request).await;
        println!("response: {response:#?}, expected_response: {expected_response:#?}");
        match response {
            Ok(resp) => assert_eq!(resp, expected_response.unwrap()),
            Err(_) => assert!(matches!(response, _expected_response)),
        }
    }

    //----------------------------------------------------------------------- Key image checks
    // Assert each key image we inserted comes back as "spent".
    for key_image in tables.key_images_iter().keys().unwrap() {
        let key_image = key_image.unwrap();
        let request = BCReadRequest::KeyImagesSpent(HashSet::from([key_image]));
        let response = reader.clone().oneshot(request).await;
        println!("response: {response:#?}, key_image: {key_image:#?}");
        assert_eq!(response.unwrap(), BCResponse::KeyImagesSpent(true));
    }

    //----------------------------------------------------------------------- Output checks
    // Create the map of amounts and amount indices.
    //
    // FIXME: There's definitely a better way to map
    // `Vec<PreRctOutputId>` -> `HashMap<u64, HashSet<u64>>`
    let (map, output_count) = {
        let mut ids = tables
            .outputs_iter()
            .keys()
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<PreRctOutputId>>();

        ids.extend(
            tables
                .rct_outputs_iter()
                .keys()
                .unwrap()
                .map(Result::unwrap)
                .map(|amount_index| PreRctOutputId {
                    amount: 0,
                    amount_index,
                }),
        );

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

    // Map `Output` -> `OutputOnChain`
    // This is the expected output from the `Response`.
    let outputs_on_chain = map
        .iter()
        .flat_map(|(amount, amount_index_set)| {
            amount_index_set.iter().map(|amount_index| {
                let id = PreRctOutputId {
                    amount: *amount,
                    amount_index: *amount_index,
                };
                id_to_output_on_chain(&id, &tables).unwrap()
            })
        })
        .collect::<Vec<OutputOnChain>>();

    // Send a request for every output we inserted before.
    let request = BCReadRequest::Outputs(map.clone());
    let response = reader.clone().oneshot(request).await;
    println!("Response::Outputs response: {response:#?}");
    let Ok(BCResponse::Outputs(response)) = response else {
        panic!("{response:#?}")
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
            assert!(outputs_on_chain.contains(&output));
        }
    }

    // Assert the amount of `Output`'s returned is as expected.
    let table_output_len = tables.outputs().len().unwrap() + tables.rct_outputs().len().unwrap();
    assert_eq!(output_count as u64, table_output_len);
    assert_eq!(output_count, response_output_count);
}

//---------------------------------------------------------------------------------------------------- Tests
/// Simply `init()` the service and then drop it.
///
/// If this test fails, something is very wrong.
#[test]
fn init_drop() {
    let (_reader, _writer, _env, _tempdir) = init_service();
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
            num_outputs: 41,
            pruned_tx_blobs: 0,
            prunable_hashes: 0,
            outputs: 111,
            prunable_tx_blobs: 0,
            rct_outputs: 0,
            tx_blobs: 3,
            tx_ids: 3,
            tx_heights: 3,
            tx_unlock_time: 1,
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
            rct_outputs: 7,
            tx_blobs: 4,
            tx_ids: 4,
            tx_heights: 4,
            tx_unlock_time: 1,
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
            rct_outputs: 1,
            tx_blobs: 1,
            tx_ids: 1,
            tx_heights: 1,
            tx_unlock_time: 1,
        },
    )
    .await;
}
