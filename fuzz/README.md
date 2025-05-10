 # Fuzz Tests
 
This folder contains the fuzz tests for crates that make up `cuprated`. To run you will need Rust and `cargo-fuzz` 
installed, the instructions for installing `cargo-fuzz` can be found here: https://rust-fuzz.github.io/book/cargo-fuzz/setup.html.

Once you have `cargo-fuzz` and have switched to the nightly compiler, you can list the possible targets with:

```
cargo fuzz list
```

Now you can pick a target to fuzz and fuzz it with:

```
CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run $TARGET -O
```

for example to fuzz the `levin_codec` target you would do:

```
CARGO_PROFILE_RELEASE_LTO=false cargo fuzz run levin_codec -O
```

`CARGO_PROFILE_RELEASE_LTO=false` is needed to disable lto, which is not supported when fuzzing, `-O` enables optimisations.

You can use `-j X` to increase the number of concurrent jobs.
