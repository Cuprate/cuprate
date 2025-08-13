# Fast sync hashes
Cuprate has a binary that generates `fast-sync` hashes and puts them into a JSON file - this file is then used by `cuprated`.

The code that does so is located at [`consensus/fast-sync`](https://github.com/Cuprate/cuprate/blob/main/consensus/fast-sync).

To create the hashes, you need a fully synced database generated from `cuprated`.

After that, build the binary that generates `fast-sync` hashes:
```bash
cargo build --release --package cuprate-fast-sync
```

Run the binary:
```bash
./target/release/create-fs-file --height $HEIGHT
```
where `$HEIGHT` is the top blockchain height.

The generated `fast_sync_hashes.json` file should be in the current directory.

This should be moved to `binaries/cuprated/src/blockchain/fast_sync/fast_sync_hashes.json`.