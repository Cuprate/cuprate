#![expect(unused_crate_dependencies, reason = "external test module")]

/*
TODO: add this test back

#![expect(clippy::allow_attributes, reason = "usage inside macro")]

use std::{
    collections::{BTreeMap, HashMap},
    future::ready,
    sync::Arc,
};

use curve25519_dalek::constants::ED25519_BASEPOINT_COMPRESSED;
use indexmap::IndexMap;
use monero_oxide::{
    io::CompressedPoint,
    transaction::{Timelock, Transaction},
};
use tower::service_fn;

use cuprate_consensus::{__private::Database, transactions::start_tx_verification};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
    OutputOnChain,
};

use cuprate_consensus_rules::HardFork;

use cuprate_test_utils::data::TX_E2D393;

fn dummy_database(outputs: BTreeMap<u64, OutputOnChain>) -> impl Database + Clone {
    let outputs = Arc::new(outputs);

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "Other database requests are not needed for this test"
    )]
    service_fn(move |req: BlockchainReadRequest| {
        ready(Ok(match req {
            BlockchainReadRequest::NumberOutputsWithAmount(_) => {
                BlockchainResponse::NumberOutputsWithAmount(HashMap::new(), [0; 32])
            }
            BlockchainReadRequest::Outputs { outputs: outs, .. } => {
                let idxs = &outs[&0];

                let mut ret = IndexMap::new();

                ret.insert(
                    0_u64,
                    idxs.iter()
                        .map(|idx| (*idx, *outputs.get(idx).unwrap()))
                        .collect::<IndexMap<_, _>>(),
                );

                let ret = OutputCache::new(ret, IndexMap::new(), IndexMap::new(), [0; 32]);

                BlockchainResponse::Outputs(ret)
            }
            BlockchainReadRequest::KeyImagesSpent(_) => BlockchainResponse::KeyImagesSpent(false),
            _ => panic!(),
        }))
    })
}

macro_rules! test_verify_valid_v2_tx {
    (
        $test_name: ident,
        $tx: ident,
        Rings: $([
            $($idx: literal: ($ring_member: literal, $commitment: literal),)+
        ],)+
        $hf: ident
    ) => {

        #[tokio::test]
        #[allow(const_item_mutation)]
        async fn $test_name() {
            let members = vec![
                $($(($idx,
                OutputOnChain {
                    height: 0,
                    time_lock: Timelock::None,
                    commitment: CompressedPoint(hex_literal::hex!($commitment)),
                    key: CompressedPoint(hex_literal::hex!($ring_member)),
                    txid: None,
                }),)+)+
            ];

            let map = BTreeMap::from_iter(members);
            let database = dummy_database(map);

            assert!(
                start_tx_verification()
                .append_txs(
                    vec![Transaction::read(&mut $tx).unwrap()]
                )
                .prepare()
                .unwrap()
                .full(10, [0; 32], u64::MAX, HardFork::$hf, database.clone(), None)
                .verify()
                .await.is_ok()
            );

            // Check verification fails if we put random ring members

            let members = vec![
                $($(($idx,
                OutputOnChain {
                    height: 0,
                    time_lock: Timelock::None,
                    commitment: CompressedPoint::from(ED25519_BASEPOINT_COMPRESSED),
                    key: CompressedPoint(hex_literal::hex!($ring_member)),
                    txid: None,
                }),)+)+
            ];

            let map = BTreeMap::from_iter(members);
            let database = dummy_database(map);

            assert!(
                start_tx_verification()
                .append_txs(
                    vec![Transaction::read(&mut $tx).unwrap()]
                )
                .prepare()
                .unwrap()
                .full(10, [0; 32], u64::MAX, HardFork::$hf, database.clone(), None)
                .verify()
                .await.is_err()
            );

        }
    };
}

test_verify_valid_v2_tx! {
    verify_tx_e2d393,
    TX_E2D393,
    Rings: [
        7567582: ("5fa4f8b160c0877476e78094d0ce4951b20f43088f6e3698fa4d3154069c7c1b", "9a41189729e8cf113cee0b126e22653f3f551227947f54fbbb16ae8d535d757d"),
        7958047: ("0febe3d139bf3db267c2efdc714ea9b42e437a5aa16e42848a835d009108fcdf", "ecca12345c02c6b0348cfa988a0d86d34e3a89cd8b53dd4ffdb860cee0eda487"),// miner amt: 3551239030364
        8224417: ("bdd1fb8a725ae15ce37bc8090925126396f87c2972d728814f2d622baa77ebf6", "24624e957c351727deadafda531f7bed433220e72dc85f8aa8d3d32cd7df42e1"),
        8225772: ("cddef0210ed3113f3362ecb7aa43003c6c3ed4bcac09dc4d9d8d015472c8a3d8", "f61b954879a0f3cc3540f0364ad108fe286162f993f4b435b42038c29d07b8c2"),
        8234785: ("4edf5a8448e133fcb7914ea161dbb8eb0057e44284d0315839d9fce4cdb063e8", "1cec1e2f88268d6f164f07f79c663bd1af09920a9254164f518faff45dd42138"),
        8247173: ("cbee0e5fa9c31689b174862a6eb0a164a2d807d2862ac0ad50c0030f0af6c5e7", "f229752b609d923cda89735ed2a42a9af6fc3e3219ac164f17d5eac4f85f391c"),
        8285361: ("f16dbd9542e7dd575c15e2c9217f5cecb6d134383e5e8416da4affab132f1ff8", "7e31ad658fff150b0ae3a9329e353522ed20dd3ac8df8cd965fa4369164857b4"),
        8308826: ("4ce2b333cc421237fc96f1a0719d4ac0892f0ff457f3a14f2e499fc045cd4714", "2f7f240e42cbd3a5f02b0b185465263b6a4c6df609dcf928314ea7ddbec3d3dc"),// miner amt: 3408911250482
        8312407: ("ead8dfb7423f5c3fa7f10663ce885d27d1b7eeb634ac05fd74d3b080440819bf", "236c3fde472978aff92aeb6e752eeb681dfdbb9a84d7e049238f7f544b85062a"),
        8314321: ("24d3dadeef6b0aff3ee7288cd391823b0020ba3fab42085f66765fc2a164f879", "bffce0393f1fc96e3d83a057208b506c9f7ad52e012e20b228918932c6c8287a"),
        8315222: ("a8b165589dffa4c31c27fb432cfdd4855b0d04102b79e439720bb80198d5b9c0", "c3febd29c1a3cc397639ff7fdb357d22a900821bef956af626651f2a916cf6f6"),
    ],
    V9
}


 */
