use std::{sync::atomic::Ordering, time::Instant};

use crossbeam::channel::Receiver;
use monero_serai::block::Block;

use crate::{
    cryptonight::CryptoNightHash, rpc::GetBlockResponse, VerifyData, RANDOMX_START_HEIGHT,
    TESTED_BLOCK_COUNT,
};

#[expect(
    clippy::needless_pass_by_value,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub(crate) fn spawn_verify_pool(thread_count: usize, top_height: u64, rx: Receiver<VerifyData>) {
    let now = Instant::now();

    for i in 0..thread_count {
        let rx = rx.clone();

        std::thread::spawn(move || {
            let mut current_seed_hash = [0; 32];
            let mut randomx_vm = None;

            loop {
                let Ok(data) = rx.recv() else {
                    println!("Exiting verify thread {i}/{thread_count}");
                    return;
                };

                let VerifyData {
                    get_block_response,
                    height,
                    seed_height,
                    seed_hash,
                } = data;

                let GetBlockResponse { blob, block_header } = get_block_response;
                let header = block_header;

                let block = match Block::read(&mut blob.as_slice()) {
                    Ok(b) => b,
                    Err(e) => panic!("{e:?}\nblob: {blob:?}, header: {header:?}"),
                };

                let pow_data = block.serialize_pow_hash();

                let (algo, pow_hash) = if height < RANDOMX_START_HEIGHT {
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

                assert_eq!(
                    header.pow_hash, pow_hash,
                    "\nheight: {height}\nheader: {header:#?}\nblock: {block:#?}",
                );

                let count = TESTED_BLOCK_COUNT.fetch_add(1, Ordering::Release) + 1;

                if std::env::var("VERBOSE").is_err() && count % 500 != 0 {
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

                println!(
                    "progress    | {count}/{top_height} ({percent:.2}%, {bps:.2} blocks/sec, {h}h {m}m {s}s left)
algo        | {algo}
seed_height | {seed_height}
seed_hash   | {seed_hash}
pow_hash    | {pow_hash}\n"
                );
            }
        });
    }
}
