# Fast sync hashes
Cuprate has a binary that generate `fast-sync` hashes and puts them into a binary blob file.

The code that does so is located at [`consensus/fast-sync`](https://github.com/Cuprate/cuprate/blob/main/consensus/fast-sync).

To create the hashes, you must need a fully synced database generated from `cuprated`.

After that, build the binary:
```bash
cargo build --release --package cuprate-fast-sync
```

Run the binary:
```bash
./target/release/cuprate-fast-sync-create-hashes --height $HEIGHT
```
where `$HEIGHT` is the top blockchain height.

The generated file should be located at `consensus/fast-sync/src/data/hashes_of_hashes`.