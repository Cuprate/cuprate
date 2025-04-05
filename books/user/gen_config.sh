#!/bin/bash

# https://rust-lang.github.io/mdBook/for_developers/preprocessors.html

# This script is called twice, the first is just to check support.
if [ "$1" == "supports" ]; then
    # return 0 - we support everything.
    exit 0;
fi

# Second call - generate config.
cargo run --bin cuprated -- --generate-config > ./books/user/Cuprated.toml

# This looks weird but mdbook hands us 2 JSON maps, we need to return the second with any edits we want to make.
# We don't want to make any edits, so we can just read & return the second JSON map straight away.
jq '.[1]'
