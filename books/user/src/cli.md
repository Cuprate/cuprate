# Command line

Command line options will override any overlapping [config](./config.md) values.

Usage: `cuprated [OPTIONS]`

<!-- TODO: automate the generation of the below table from `./cuprated --help` -->

| Option | Description | Default | Possible values |
|--------|-------------|---------|-----------------|
| `--network <NETWORK>` | The network to run on | `mainnet` | `mainnet`, `testnet`, `stagenet`
| `--outbound-connections <OUTBOUND_CONNECTIONS>` | The amount of outbound clear-net connections to maintain | `64` |
| `--config-file <CONFIG_FILE>` | The PATH of the `cuprated` config file | `Cuprated.toml` |
| `--generate-config` | Generate a config file and print it to stdout | |
| `--skip-config-warning` | Stops the missing config warning and startup delay if a config file is missing | |
| `-v`, `--version` | Print misc version information in JSON | |
| `-h`, `--help` | Print help | |