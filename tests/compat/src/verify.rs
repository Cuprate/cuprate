use std::{
    collections::HashSet,
    num::{NonZeroU64, NonZeroUsize},
    sync::{atomic::Ordering, LazyLock, Mutex},
    time::Instant,
};

use crossbeam::channel::Receiver;

use crate::{
    constants::{RANDOMX_START_HEIGHT, TESTED_BLOCK_COUNT, TESTED_TX_COUNT},
    cryptonight::CryptoNightHash,
    types::{BlockHeader, GetBlockResponse, RpcBlockData, RpcTxData},
};

#[expect(
    clippy::needless_pass_by_value,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::significant_drop_tightening
)]
pub fn spawn_verify_pool(
    thread_count: NonZeroUsize,
    update: NonZeroU64,
    top_height: u64,
    rx: Receiver<RpcBlockData>,
) {
    let now = Instant::now();

    for i in 0..thread_count.get() {
        let rx = rx.clone();

        std::thread::spawn(move || {
            let mut current_seed_hash = [0; 32];
            let mut randomx_vm = None;

            loop {
                let Ok(data) = rx.recv() else {
                    println!("Exiting verify thread {i}/{thread_count}");
                    return;
                };

                // Panic info.
                let p = format!("data: {data:#?}");

                let RpcBlockData {
                    get_block_response,
                    block,
                    seed_height,
                    seed_hash,
                    txs,
                } = data;
                let GetBlockResponse { blob, block_header } = get_block_response;
                let BlockHeader {
                    block_weight,
                    hash,
                    pow_hash,
                    height,
                    major_version,
                    minor_version,
                    miner_tx_hash,
                    nonce,
                    num_txes,
                    prev_hash,
                    reward,
                    timestamp,
                } = block_header;

                // Test block properties.
                assert_eq!(blob, block.serialize(), "{p:#?}");

                assert!(
                    !block.miner_transaction.prefix().outputs.is_empty(),
                    "miner_tx has no outputs\n{p:#?}"
                );

                let block_reward = block
                    .miner_transaction
                    .prefix()
                    .outputs
                    .iter()
                    .map(|o| o.amount.unwrap())
                    .sum::<u64>();
                assert_ne!(block_reward, 0, "block reward is 0\n{p:#?}");

                let total_block_weight = txs
                    .iter()
                    .map(|RpcTxData { tx, .. }| tx.weight())
                    .sum::<usize>();

                // Test all transactions are unique.
                {
                    static TX_SET: LazyLock<Mutex<HashSet<[u8; 32]>>> =
                        LazyLock::new(|| Mutex::new(HashSet::new()));

                    let mut tx_set = TX_SET.lock().unwrap();

                    for tx_hash in txs.iter().map(|RpcTxData { tx_hash, .. }| tx_hash) {
                        assert!(
                            tx_set.insert(*tx_hash),
                            "duplicated tx_hash: {}, {p:#?}",
                            hex::encode(tx_hash),
                        );
                    }
                }

                // Test transaction properties.
                for RpcTxData {
                    tx,
                    tx_blob,
                    tx_hash,
                } in txs
                {
                    assert_eq!(tx_hash, tx.hash(), "{p:#?}, tx: {tx:#?}");
                    assert_ne!(tx.weight(), 0, "{p:#?}, tx: {tx:#?}");
                    assert!(!tx.prefix().inputs.is_empty(), "{p:#?}, tx: {tx:#?}");
                    assert_eq!(tx_blob, tx.serialize(), "{p:#?}, tx: {tx:#?}");
                    assert!(matches!(tx.version(), 1 | 2), "{p:#?}, tx: {tx:#?}");
                }

                // Test block fields are correct.
                assert_eq!(block_weight, total_block_weight, "{p:#?}");
                assert_ne!(block.miner_transaction.weight(), 0, "{p:#?}");
                assert_eq!(hash, block.hash(), "{p:#?}");
                assert_eq!(
                    height,
                    u64::try_from(block.number().unwrap()).unwrap(),
                    "{p:#?}"
                );
                assert_eq!(major_version, block.header.hardfork_version, "{p:#?}");
                assert_eq!(minor_version, block.header.hardfork_signal, "{p:#?}");
                assert_eq!(miner_tx_hash, block.miner_transaction.hash(), "{p:#?}");
                assert_eq!(nonce, block.header.nonce, "{p:#?}");
                assert_eq!(num_txes, block.transactions.len(), "{p:#?}");
                assert_eq!(prev_hash, block.header.previous, "{p:#?}");
                assert_eq!(reward, block_reward, "{p:#?}");
                assert_eq!(timestamp, block.header.timestamp, "{p:#?}");

                //
                let pow_data = block.serialize_pow_hash();

                let (algo, calculated_pow_hash) = if height < RANDOMX_START_HEIGHT {
                    CryptoNightHash::hash(&pow_data, height)
                } else {
                    if current_seed_hash != seed_hash {
                        randomx_vm = None;
                    }

                    let randomx_vm = randomx_vm.get_or_insert_with(|| {
                        current_seed_hash = seed_hash;
                        // crate::randomx::randomx_vm_optimized(&seed_hash)
                        crate::randomx::randomx_vm_default(&seed_hash)
                    });

                    let pow_hash = randomx_vm
                        .calculate_hash(&pow_data)
                        .unwrap()
                        .try_into()
                        .unwrap();

                    ("randomx", pow_hash)
                };

                assert_eq!(calculated_pow_hash, pow_hash, "{p:#?}",);

                let count = TESTED_BLOCK_COUNT.fetch_add(1, Ordering::Release) + 1;
                let total_tx_count = TESTED_TX_COUNT.fetch_add(num_txes, Ordering::Release) + 1;

                if count % update.get() != 0 {
                    continue;
                }

                let pow_hash = hex::encode(pow_hash);
                let seed_hash = hex::encode(seed_hash);
                let percent = (count as f64 / top_height as f64) * 100.0;

                let elapsed = now.elapsed().as_secs_f64();
                let secs_per_hash = elapsed / count as f64;
                let bps = count as f64 / elapsed;
                let remaining_secs = (top_height as f64 - count as f64) * secs_per_hash;
                let h = (remaining_secs / 60.0 / 60.0) as u64;
                let m = (remaining_secs / 60.0 % 60.0) as u64;
                let s = (remaining_secs % 60.0) as u64;

                let block_hash = hex::encode(hash);
                let miner_tx_hash = hex::encode(miner_tx_hash);
                let prev_hash = hex::encode(prev_hash);
                let miner_tx_weight = block.miner_transaction.weight();

                println!("progress        | {count}/{top_height} ({percent:.2}%, {algo}, {bps:.2} blocks/sec, {h}h {m}m {s}s left)
seed_hash       | {seed_hash}
pow_hash        | {pow_hash}
block_hash      | {block_hash}
miner_tx_hash   | {miner_tx_hash}
prev_hash       | {prev_hash}
reward          | {reward}
timestamp       | {timestamp}
nonce           | {nonce}
total_tx_count  | {total_tx_count}
height          | {height}
seed_height     | {seed_height}
block_weight    | {block_weight}
miner_tx_weight | {miner_tx_weight}
major_version   | {major_version}
minor_version   | {minor_version}
num_txes        | {num_txes}\n",
                );
            }
        });
    }
}
