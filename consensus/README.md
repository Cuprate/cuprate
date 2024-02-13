# Consensus Rules

This folder contains 2 crates: `cuprate-consensus-rules` (rules) and `cuprate-consensus`. `cuprate-consensus-rules` contains the raw-rules
and is built to be a more flexible library which requires the user to give the correct data and do minimal calculations, `cuprate-consensus`
on the other hand contains multiple tower::Services that handle tx/ block verification as a whole with a `context` service that
keeps track of blockchain state. `cuprate-consensus` uses `cuprate-consensus-rules` internally.

If you are looking to use monero consensus rules it's recommended you try to integrate `cuprate-consensus` and fall back to
`cuprate-consensus-rules` if you need more flexibility.

## scan_chain

`cuprate-consensus` contains a binary,`scan_chain`, which uses multiple RPC connections to scan the blockchain and verify it against the
consensus rules. It keeps track of minimal data and uses the RPC connection to get blocks/transactions/outputs.

`scan_chain` was not built for wide usage, so you may find issues, if you do, open an issue in Cuprates issue tracker and or join our matrix
room for help. `scan_chain` has only been verified on `x86_64-unknown-linux-gnu`.

`scan_chain` will take at least a day for stagenet and testnet and 6 for mainnet but expect it to be longer. If you are just looking to verify
previous transactions it may be worth using `monerod` with `--fast-block-sync 0` this will probably be faster to complete and you will have a
usable node at the end!

### How to run

First you will need to install Rust/Cargo: https://www.rust-lang.org/tools/install

Next you need to clone Cuprates git repo, enter the root of Cuprate, then run:

```
cargo run --features binaries --bin scan_chain -r 
```

If you want to pass in options you need to add `--` then the option(s), so to list the options do:

```
cargo run --features binaries --bin scan_chain -r -- --help
```