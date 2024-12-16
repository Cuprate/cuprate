use std::{
    collections::HashSet,
    num::{NonZeroU64, NonZeroUsize},
    sync::{atomic::Ordering, LazyLock, Mutex},
    time::Instant,
};

use crossbeam::channel::Receiver;
use monero_serai::block::Block;
use randomx_rs::RandomXVM;

use crate::{
    constants::{RANDOMX_START_HEIGHT, TESTED_BLOCK_COUNT, TESTED_TX_COUNT},
    cryptonight::CryptoNightHash,
    types::{BlockHeader, GetBlockResponse, RpcBlockData, RpcTxData},
};

struct Verifier {
    id: usize,
    now: Instant,
    thread_count: NonZeroUsize,
    update: NonZeroU64,
    top_height: u64,
    rx: Receiver<RpcBlockData>,
    seed_hash: [u8; 32],
    timestamp: u64,
    randomx_vm: Option<RandomXVM>,
}

#[expect(clippy::needless_pass_by_value)]
pub fn spawn_verify_pool(
    thread_count: NonZeroUsize,
    update: NonZeroU64,
    top_height: u64,
    rx: Receiver<RpcBlockData>,
) {
    let now = Instant::now();

    for id in 0..thread_count.get() {
        let rx = rx.clone();
        std::thread::spawn(move || {
            Verifier {
                id,
                now,
                thread_count,
                update,
                top_height,
                rx,
                seed_hash: [0; 32],
                timestamp: 0,
                randomx_vm: None,
            }
            .loop_listen_verify();
        });
    }
}

impl Verifier {
    fn loop_listen_verify(mut self) {
        loop {
            let Ok(data) = self.rx.recv() else {
                println!("Exiting verify thread {}/{}", self.id, self.thread_count);
                return;
            };

            self.verify(data);
        }
    }

    fn verify(&mut self, data: RpcBlockData) {
        //----------------------------------------------- Create panic info.
        let p = format!("data: {data:#?}");

        //----------------------------------------------- Extract data.
        let RpcBlockData {
            get_block_response,
            block,
            seed_height,
            seed_hash,
            txs,
        } = data;
        let GetBlockResponse { blob, block_header } = get_block_response;

        //----------------------------------------------- Calculate some data.
        let calculated_block_reward = block
            .miner_transaction
            .prefix()
            .outputs
            .iter()
            .map(|o| o.amount.unwrap())
            .sum::<u64>();
        let calculated_block_weight = txs
            .iter()
            .map(|RpcTxData { tx, .. }| tx.weight())
            .sum::<usize>();
        let calculated_pow_data = block.serialize_pow_hash();
        let miner_tx_weight = block.miner_transaction.weight();

        //----------------------------------------------- Verify.
        Self::verify_block_properties(&blob, &block, calculated_block_reward, &p);
        Self::verify_all_transactions_are_unique(&txs, &p);
        Self::verify_transaction_properties(txs, &p);

        self.verify_block_fields(
            calculated_block_weight,
            calculated_block_reward,
            &block,
            &p,
            block_header,
        );

        let algo = self.verify_pow(
            block_header.height,
            seed_hash,
            block_header.pow_hash,
            &calculated_pow_data,
            &p,
        );

        //----------------------------------------------- Print progress.
        self.print_progress(algo, seed_height, miner_tx_weight, block_header);
    }

    fn verify_block_properties(
        block_blob: &[u8],
        block: &Block,
        calculated_block_reward: u64,
        p: &str,
    ) {
        // Test block properties.
        assert_eq!(block_blob, block.serialize(), "{p}");

        assert!(
            !block.miner_transaction.prefix().outputs.is_empty(),
            "miner_tx has no outputs\n{p}"
        );

        assert_ne!(calculated_block_reward, 0, "block reward is 0\n{p}");
    }

    #[expect(clippy::significant_drop_tightening)]
    fn verify_all_transactions_are_unique(txs: &[RpcTxData], p: &str) {
        static TX_SET: LazyLock<Mutex<HashSet<[u8; 32]>>> =
            LazyLock::new(|| Mutex::new(HashSet::new()));

        let mut tx_set = TX_SET.lock().unwrap();

        for tx_hash in txs.iter().map(|RpcTxData { tx_hash, .. }| tx_hash) {
            assert!(
                tx_set.insert(*tx_hash),
                "duplicated tx_hash: {}, {p}",
                hex::encode(tx_hash),
            );
        }
    }

    fn verify_transaction_properties(txs: Vec<RpcTxData>, p: &str) {
        // Test transaction properties.
        for RpcTxData {
            tx,
            tx_blob,
            tx_hash,
        } in txs
        {
            assert_eq!(tx_hash, tx.hash(), "{p}, tx: {tx:#?}");
            assert_ne!(tx.weight(), 0, "{p}, tx: {tx:#?}");
            assert!(!tx.prefix().inputs.is_empty(), "{p}, tx: {tx:#?}");
            assert_eq!(tx_blob, tx.serialize(), "{p}, tx: {tx:#?}");
            assert!(matches!(tx.version(), 1 | 2), "{p}, tx: {tx:#?}");
        }
    }

    fn verify_block_fields(
        &mut self,
        calculated_block_weight: usize,
        calculated_block_reward: u64,
        block: &Block,
        p: &str,
        BlockHeader {
            block_weight,
            hash,
            pow_hash: _,
            height,
            major_version,
            minor_version,
            miner_tx_hash,
            nonce,
            num_txes,
            prev_hash,
            reward,
            timestamp,
        }: BlockHeader,
    ) {
        // Test block fields are correct.
        assert_eq!(block_weight, calculated_block_weight, "{p}");
        assert_ne!(block.miner_transaction.weight(), 0, "{p}");
        assert_eq!(hash, block.hash(), "{p}");
        assert_eq!(
            height,
            u64::try_from(block.number().unwrap()).unwrap(),
            "{p}"
        );
        assert_eq!(major_version, block.header.hardfork_version, "{p}");
        assert_eq!(minor_version, block.header.hardfork_signal, "{p}");
        assert_eq!(miner_tx_hash, block.miner_transaction.hash(), "{p}");
        assert_eq!(nonce, block.header.nonce, "{p}");
        assert_eq!(num_txes, block.transactions.len(), "{p}");
        assert_eq!(prev_hash, block.header.previous, "{p}");
        assert_eq!(reward, calculated_block_reward, "{p}");
        assert_eq!(timestamp, block.header.timestamp, "{p}");

        if timestamp != 0 {
            assert!(timestamp > self.timestamp, "{p}");
            self.timestamp = timestamp;
        }
    }

    fn verify_pow(
        &mut self,
        height: u64,
        seed_hash: [u8; 32],
        pow_hash: [u8; 32],
        calculated_pow_data: &[u8],
        p: &str,
    ) -> &'static str {
        let (algo, calculated_pow_hash) = if height < RANDOMX_START_HEIGHT {
            CryptoNightHash::hash(calculated_pow_data, height)
        } else {
            if self.seed_hash != seed_hash {
                self.randomx_vm = None;
            }

            let randomx_vm = self.randomx_vm.get_or_insert_with(|| {
                self.seed_hash = seed_hash;
                // crate::randomx::randomx_vm_optimized(&seed_hash)
                crate::randomx::randomx_vm_default(&seed_hash)
            });

            let pow_hash = randomx_vm
                .calculate_hash(calculated_pow_data)
                .unwrap()
                .try_into()
                .unwrap();

            ("randomx", pow_hash)
        };

        assert_eq!(calculated_pow_hash, pow_hash, "{p}",);

        algo
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn print_progress(
        &self,
        algo: &'static str,
        seed_height: u64,
        miner_tx_weight: usize,
        BlockHeader {
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
        }: BlockHeader,
    ) {
        let count = TESTED_BLOCK_COUNT.fetch_add(1, Ordering::Release) + 1;
        let total_tx_count = TESTED_TX_COUNT.fetch_add(num_txes, Ordering::Release) + 1;

        if count % self.update.get() != 0 {
            return;
        }

        let top_height = self.top_height;

        let percent = (count as f64 / top_height as f64) * 100.0;

        let elapsed = self.now.elapsed().as_secs_f64();
        let secs_per_hash = elapsed / count as f64;
        let bps = count as f64 / elapsed;
        let remaining_secs = (top_height as f64 - count as f64) * secs_per_hash;
        let h = (remaining_secs / 60.0 / 60.0) as u64;
        let m = (remaining_secs / 60.0 % 60.0) as u64;
        let s = (remaining_secs % 60.0) as u64;

        let pow_hash = hex::encode(pow_hash);
        let seed_hash = hex::encode(self.seed_hash);
        let block_hash = hex::encode(hash);
        let miner_tx_hash = hex::encode(miner_tx_hash);
        let prev_hash = hex::encode(prev_hash);

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
}
