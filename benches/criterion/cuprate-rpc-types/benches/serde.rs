//! This module contains benchmarks for any
//!
//! - non-trivial
//! - manual
//! - common
//!
//! type with a `serde` implementation.
//!
//! Types with the standard `serde` derive implementation are not included.

#![allow(unused_attributes, unused_crate_dependencies)]

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use function_name::named;
use serde_json::{from_str, to_string};

use cuprate_rpc_types::{
    json::{
        CalcPowRequest, GetBlockHeadersRangeResponse, GetBlockResponse, GetBlockTemplateResponse,
        GetConnectionsResponse, GetInfoResponse, GetLastBlockHeaderResponse, SyncInfoResponse,
    },
    misc::TxEntry,
};

/// Generate [`from_str`] and [`to_string`] benchmarks for `serde` types.
macro_rules! generate_serde_benchmarks {
    (
        $(
            // The type to test => JSON of that type
            $t:ty => $t_example:expr
        ),* $(,)?
    ) => { paste::paste! {
        // Generate the benchmarking functions.
        $(
            #[named]
            fn [<serde_from_str_ $t:snake>](c: &mut Criterion) {
                c.bench_function(function_name!(), |b| {
                    b.iter(|| {
                        drop(from_str::<$t>(
                            black_box($t_example)
                        ).unwrap());
                    });
                });
            }

            #[named]
            fn [<serde_to_string_ $t:snake>](c: &mut Criterion) {
                let t = $t::default();

                c.bench_function(function_name!(), |b| {
                    b.iter_batched(
                        || t.clone(),
                        |t| drop(to_string(black_box(&t)).unwrap()),
                        BatchSize::SmallInput,
                    );
                });
            }

        )*

        // Enable all the benchmark functions created in this macro.
        criterion_group! {
            name = benches;
            config = Criterion::default();
            targets =
            $(
                [<serde_from_str_ $t:snake>],
                [<serde_to_string_ $t:snake>],
            )*
        }
        criterion_main!(benches);
    }};
}

// FIXME: these could be taken from `cuprate-test-utils::rpc::data::json` but
// those are wrapped in JSON-RPC, so we have to copy+paste the inner portion here.
generate_serde_benchmarks! {
    // Custom serde types.
    TxEntry => r#"{"as_hex":"","as_json":"","double_spend_seen":false,"prunable_as_hex":"","prunable_hash":"","pruned_as_hex":"","received_timestamp":0,"relayed":false,"tx_hash":"","in_pool":false}"#,
    // Distribution => "TODO: enable after type is finalized"

    // Common types or heavy types (heap types, many fields, etc).
    GetLastBlockHeaderResponse => r#"{"block_header":{"block_size":200419,"block_weight":200419,"cumulative_difficulty":366125734645190820,"cumulative_difficulty_top64":0,"depth":0,"difficulty":282052561854,"difficulty_top64":0,"hash":"57238217820195ac4c08637a144a885491da167899cf1d20e8e7ce0ae0a3434e","height":3195020,"long_term_weight":200419,"major_version":16,"miner_tx_hash":"7a42667237d4f79891bb407c49c712a9299fb87fce799833a7b633a3a9377dbd","minor_version":16,"nonce":1885649739,"num_txes":37,"orphan_status":false,"pow_hash":"","prev_hash":"22c72248ae9c5a2863c94735d710a3525c499f70707d1c2f395169bc5c8a0da3","reward":615702960000,"timestamp":1721245548,"wide_cumulative_difficulty":"0x514bd6a74a7d0a4","wide_difficulty":"0x41aba48bbe"},"credits":0,"status":"OK","top_hash":"","untrusted":false}"#,
    CalcPowRequest => r#"{"major_version":14,"height":2286447,"block_blob":"0e0ed286da8006ecdc1aab3033cf1716c52f13f9d8ae0051615a2453643de94643b550d543becd0000000002abc78b0101ffefc68b0101fcfcf0d4b422025014bb4a1eade6622fd781cb1063381cad396efa69719b41aa28b4fce8c7ad4b5f019ce1dc670456b24a5e03c2d9058a2df10fec779e2579753b1847b74ee644f16b023c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000051399a1bc46a846474f5b33db24eae173a26393b976054ee14f9feefe99925233802867097564c9db7a36af5bb5ed33ab46e63092bd8d32cef121608c3258edd55562812e21cc7e3ac73045745a72f7d74581d9a0849d6f30e8b2923171253e864f4e9ddea3acb5bc755f1c4a878130a70c26297540bc0b7a57affb6b35c1f03d8dbd54ece8457531f8cba15bb74516779c01193e212050423020e45aa2c15dcb","seed_hash":"d432f499205150873b2572b5f033c9c6e4b7c6f3394bd2dd93822cd7085e7307"}"#,
    SyncInfoResponse => r#"{"credits":0,"height":3195157,"next_needed_pruning_seed":0,"overview":"[]","peers":[{"info":{"address":"142.93.128.65:44986","address_type":1,"avg_download":1,"avg_upload":1,"connection_id":"a5803c4c2dac49e7b201dccdef54c862","current_download":2,"current_upload":1,"height":3195157,"host":"142.93.128.65","incoming":true,"ip":"142.93.128.65","live_time":18,"local_ip":false,"localhost":false,"peer_id":"6830e9764d3e5687","port":"44986","pruning_seed":0,"recv_count":20340,"recv_idle_time":0,"rpc_credits_per_hash":0,"rpc_port":18089,"send_count":32235,"send_idle_time":6,"state":"normal","support_flags":1}},{"info":{"address":"4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion:18083","address_type":4,"avg_download":0,"avg_upload":0,"connection_id":"277f7c821bc546878c8bd29977e780f5","current_download":0,"current_upload":0,"height":3195157,"host":"4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion","incoming":false,"ip":"","live_time":2246,"local_ip":false,"localhost":false,"peer_id":"0000000000000001","port":"","pruning_seed":389,"recv_count":65164,"recv_idle_time":15,"rpc_credits_per_hash":0,"rpc_port":0,"send_count":99120,"send_idle_time":15,"state":"normal","support_flags":0}}],"status":"OK","target_height":0,"top_hash":"","untrusted":false}"#,
    GetInfoResponse => r#"{"adjusted_time":1721245289,"alt_blocks_count":16,"block_size_limit":600000,"block_size_median":300000,"block_weight_limit":600000,"block_weight_median":300000,"bootstrap_daemon_address":"","busy_syncing":false,"credits":0,"cumulative_difficulty":366127702242611947,"cumulative_difficulty_top64":0,"database_size":235169075200,"difficulty":280716748706,"difficulty_top64":0,"free_space":30521749504,"grey_peerlist_size":4996,"height":3195028,"height_without_bootstrap":3195028,"incoming_connections_count":62,"mainnet":true,"nettype":"mainnet","offline":false,"outgoing_connections_count":1143,"restricted":false,"rpc_connections_count":1,"stagenet":false,"start_time":1720462427,"status":"OK","synchronized":true,"target":120,"target_height":0,"testnet":false,"top_block_hash":"bdf06d18ed1931a8ee62654e9b6478cc459bc7072628b8e36f4524d339552946","top_hash":"","tx_count":43205750,"tx_pool_size":12,"untrusted":false,"update_available":false,"version":"0.18.3.3-release","was_bootstrap_ever_used":false,"white_peerlist_size":1000,"wide_cumulative_difficulty":"0x514bf349299d2eb","wide_difficulty":"0x415c05a7a2"}"#,
    GetBlockResponse => r#"{"blob":"1010c58bab9b06b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7807e07f502cef8a70101ff92f8a7010180e0a596bb1103d7cbf826b665d7a532c316982dc8dbc24f285cbc18bbcc27c7164cd9b3277a85d034019f629d8b36bd16a2bfce3ea80c31dc4d8762c67165aec21845494e32b7582fe00211000000297a787a000000000000000000000000","block_header":{"block_size":106,"block_weight":106,"cumulative_difficulty":236046001376524168,"cumulative_difficulty_top64":0,"depth":443517,"difficulty":313732272488,"difficulty_top64":0,"hash":"43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428","height":2751506,"long_term_weight":176470,"major_version":16,"miner_tx_hash":"e49b854c5f339d7410a77f2a137281d8042a0ffc7ef9ab24cd670b67139b24cd","minor_version":16,"nonce":4110909056,"num_txes":0,"orphan_status":false,"pow_hash":"","prev_hash":"b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7","reward":600000000000,"timestamp":1667941829,"wide_cumulative_difficulty":"0x3469a966eb2f788","wide_difficulty":"0x490be69168"},"credits":0,"json":"{\n  \"major_version\": 16, \n  \"minor_version\": 16, \n  \"timestamp\": 1667941829, \n  \"prev_id\": \"b27bdecfc6cd0a46172d136c08831cf67660377ba992332363228b1b722781e7\", \n  \"nonce\": 4110909056, \n  \"miner_tx\": {\n    \"version\": 2, \n    \"unlock_time\": 2751566, \n    \"vin\": [ {\n        \"gen\": {\n          \"height\": 2751506\n        }\n      }\n    ], \n    \"vout\": [ {\n        \"amount\": 600000000000, \n        \"target\": {\n          \"tagged_key\": {\n            \"key\": \"d7cbf826b665d7a532c316982dc8dbc24f285cbc18bbcc27c7164cd9b3277a85\", \n            \"view_tag\": \"d0\"\n          }\n        }\n      }\n    ], \n    \"extra\": [ 1, 159, 98, 157, 139, 54, 189, 22, 162, 191, 206, 62, 168, 12, 49, 220, 77, 135, 98, 198, 113, 101, 174, 194, 24, 69, 73, 78, 50, 183, 88, 47, 224, 2, 17, 0, 0, 0, 41, 122, 120, 122, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0\n    ], \n    \"rct_signatures\": {\n      \"type\": 0\n    }\n  }, \n  \"tx_hashes\": [ ]\n}","miner_tx_hash":"e49b854c5f339d7410a77f2a137281d8042a0ffc7ef9ab24cd670b67139b24cd","status":"OK","top_hash":"","untrusted":false}"#,
    GetConnectionsResponse => r#"{"connections":[{"address":"3evk3kezfjg44ma6tvesy7rbxwwpgpympj45xar5fo4qajrsmkoaqdqd.onion:18083","address_type":4,"avg_download":0,"avg_upload":0,"connection_id":"22ef856d0f1d44cc95e84fecfd065fe2","current_download":0,"current_upload":0,"height":3195026,"host":"3evk3kezfjg44ma6tvesy7rbxwwpgpympj45xar5fo4qajrsmkoaqdqd.onion","incoming":false,"ip":"","live_time":76651,"local_ip":false,"localhost":false,"peer_id":"0000000000000001","port":"","pruning_seed":0,"recv_count":240328,"recv_idle_time":34,"rpc_credits_per_hash":0,"rpc_port":0,"send_count":3406572,"send_idle_time":30,"state":"normal","support_flags":0},{"address":"4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion:18083","address_type":4,"avg_download":0,"avg_upload":0,"connection_id":"c7734e15936f485a86d2b0534f87e499","current_download":0,"current_upload":0,"height":3195024,"host":"4iykytmumafy5kjahdqc7uzgcs34s2vwsadfjpk4znvsa5vmcxeup2qd.onion","incoming":false,"ip":"","live_time":76755,"local_ip":false,"localhost":false,"peer_id":"0000000000000001","port":"","pruning_seed":389,"recv_count":237657,"recv_idle_time":120,"rpc_credits_per_hash":0,"rpc_port":0,"send_count":3370566,"send_idle_time":120,"state":"normal","support_flags":0}],"status":"OK","untrusted":false}"#,
    GetBlockTemplateResponse => r#"{"blockhashing_blob":"1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a00000000e0c20372be23d356347091025c5b5e8f2abf83ab618378565cce2b703491523401","blocktemplate_blob":"1010f4bae0b4069d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a0000000002c681c30101ff8a81c3010180e0a596bb11033b7eedf47baf878f3490cb20b696079c34bd017fe59b0d070e74d73ffabc4bb0e05f011decb630f3148d0163b3bd39690dde4078e4cfb69fecf020d6278a27bad10c58023c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":283305047039,"difficulty_top64":0,"expected_reward":600000000000,"height":3195018,"next_seed_hash":"","prev_hash":"9d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a","reserved_offset":131,"seed_hash":"e2aa0b7b55042cd48b02e395d78fa66a29815ccc1584e38db2d1f0e8485cd44f","seed_height":3194880,"status":"OK","untrusted":false,"wide_difficulty":"0x41f64bf3ff"}"#,
    GetBlockHeadersRangeResponse => r#"{"credits":0,"headers":[{"block_size":301413,"block_weight":301413,"cumulative_difficulty":13185267971483472,"cumulative_difficulty_top64":0,"depth":1649024,"difficulty":134636057921,"difficulty_top64":0,"hash":"86d1d20a40cefcf3dd410ff6967e0491613b77bf73ea8f1bf2e335cf9cf7d57a","height":1545999,"long_term_weight":301413,"major_version":6,"miner_tx_hash":"9909c6f8a5267f043c3b2b079fb4eacc49ef9c1dee1c028eeb1a259b95e6e1d9","minor_version":6,"nonce":3246403956,"num_txes":20,"orphan_status":false,"pow_hash":"","prev_hash":"0ef6e948f77b8f8806621003f5de24b1bcbea150bc0e376835aea099674a5db5","reward":5025593029981,"timestamp":1523002893,"wide_cumulative_difficulty":"0x2ed7ee6db56750","wide_difficulty":"0x1f58ef3541"},{"block_size":13322,"block_weight":13322,"cumulative_difficulty":13185402687569710,"cumulative_difficulty_top64":0,"depth":1649023,"difficulty":134716086238,"difficulty_top64":0,"hash":"b408bf4cfcd7de13e7e370c84b8314c85b24f0ba4093ca1d6eeb30b35e34e91a","height":1546000,"long_term_weight":13322,"major_version":7,"miner_tx_hash":"7f749c7c64acb35ef427c7454c45e6688781fbead9bbf222cb12ad1a96a4e8f6","minor_version":7,"nonce":3737164176,"num_txes":1,"orphan_status":false,"pow_hash":"","prev_hash":"86d1d20a40cefcf3dd410ff6967e0491613b77bf73ea8f1bf2e335cf9cf7d57a","reward":4851952181070,"timestamp":1523002931,"wide_cumulative_difficulty":"0x2ed80dcb69bf2e","wide_difficulty":"0x1f5db457de"}],"status":"OK","top_hash":"","untrusted":false}"#
}
