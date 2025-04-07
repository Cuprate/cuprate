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
| `--version` | Print misc version information in JSON | |
| `--help` | Print help | |

## `--version`
The `--version` flag outputs the following info in JSON.

| Field                   | Type   | Description |
|-------------------------|--------|-------------|
| `major_version`         | Number | Major version of `cuprated`                           |
| `minor_version`         | Number | Minor version of `cuprated`                           |
| `patch_version`         | Number | Patch version of `cuprated`                           |
| `rpc_major_version`     | Number | Major RPC version (follows `monerod`)                 |
| `rpc_minor_version`     | Number | minor RPC version (follows `monerod`)                 |
| `rpc_version`           | Number | RPC version (follows `monerod`)                       |
| `hardfork`              | Number | Current hardfork version                              |
| `blockchain_db_version` | Number | Blockchain database version (separate from `monerod`) |
| `semantic_version`      | String | Semantic version of `cuprated`                        |
| `build`                 | String | Build of `cuprated`, either `debug` or `release`      |
| `commit`                | String | `git` commit hash of `cuprated`                       |
| `killswitch_timestamp`  | Number | Timestamp at which `cuprated`'s killswitch activates  |