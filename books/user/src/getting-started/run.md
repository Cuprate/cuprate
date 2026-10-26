# Running
To run `cuprated`:

## General distributions
```bash
./cuprated --config-file Cuprated.toml
```

## NixOS
During first run `patchelf --add-rpath "$CUPRATE_CXX_LIB_DIR" ./target/release/cuprated` Then you may continue as you would on a General distributioon (see above)

`cuprated` can be ran without a config file, although it is recommended to use one; see [`Configuration`](../config.md) for a default config file.
