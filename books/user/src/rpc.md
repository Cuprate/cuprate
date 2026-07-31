# RPC

> **⚠️ Warning ⚠️**
>
> Cuprate is still experimental software.
>
> Consider sandboxing `cuprated` before publicly exposing restricted RPC.

`monerod`'s daemon RPC has 3 kinds of interfaces:
1. [JSON-RPC 2.0](https://www.jsonrpc.org) methods called at the `/json_rpc` endpoint, e.g. [`get_block`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block)
1. JSON endpoints, e.g. [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height)
1. Binary endpoints, e.g. [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin)

<!-- TODO: explain the binary format -->

`cuprated`'s RPC aims to mirror `monerod`'s as much as it can. The end-goal is compatibility with common use-cases such as wallet software.

This section contains the development status of endpoints/methods in `cuprated`.

| Status | Meaning |
|--------|---------|
| 🟢     | Enabled and tested
| 🟣     | Enabled but has differences waiting to be resolved
| 🟠     | Enabled but not fully tested
| ⚪     | Not enabled yet
| ⚫     | Not planned to be supported

<!-- NOTE: Sort methods/endpoints A-Z -->

## JSON-RPC methods
| Method                         | Status | Notes   |
|--------------------------------|--------|---------|
| `add_aux_pow`                  | ⚪     |
| `banned`                       | ⚪     |
| `calc_pow`                     | ⚪     |
| `flush_cache`                  | ⚫     | `cuprated` does not require this method
| `flush_txpool`                 | ⚪     |
| `generateblocks`               | ⚪     |
| `get_alternate_chains`         | 🟠     |
| `get_bans`                     | ⚪     |
| `get_block`                    | 🟠     |
| `get_block_count`              | 🟠     |
| `get_block_header_by_hash`     | 🟠     |
| `get_block_header_by_height`   | 🟠     |
| `get_block_headers_range`      | 🟠     |
| `get_block_template`           | ⚪     |
| `get_coinbase_tx_sum`          | 🟠     |
| `get_connections`              | ⚪     |
| `get_fee_estimate`             | 🟠     |
| `get_info`                     | 🟣     | Some peer and bootstrap fields are placeholders |
| `get_last_block_header`        | 🟠     |
| `get_miner_data`               | 🟣     |
| `get_output_distribution`      | 🟠     |
| `get_output_histogram`         | 🟠     |
| `get_tx_ids_loose`             | ⚫     |
| `get_txpool_backlog`           | 🟠     |
| `get_version`                  | 🟠     |
| `hard_fork_info`               | 🟠     |
| `on_get_block_hash`            | 🟠     |
| `prune_blockchain`             | ⚫     |
| `relay_tx`                     | ⚪     |
| `set_bans`                     | ⚪     |
| `submit_block`                 | 🟠     |
| `sync_info`                    | ⚪     |

## JSON endpoints
Aliases are listed with their canonical endpoint.

| Endpoint                       | Status | Notes |
|--------------------------------|--------|-------|
| `/get_alt_blocks_hashes`       | ⚪     |
| `/get_height`, `/getheight`    | 🟠     |
| `/get_info`, `/getinfo`        | 🟣     | Some peer and bootstrap fields are placeholders |
| `/get_limit`                   | ⚪     |
| `/get_net_stats`               | ⚪     |
| `/get_outs`                    | ⚪     |
| `/get_peer_list`               | ⚪     |
| `/get_public_nodes`            | ⚪     |
| `/get_transaction_pool`        | ⚪     |
| `/get_transaction_pool_hashes` | 🟠     |
| `/get_transaction_pool_stats`  | ⚪     |
| `/get_transactions`, `/gettransactions` | 🟣     | `decode_as_json` is unsupported |
| `/in_peers`                    | ⚪     |
| `/is_key_image_spent`          | 🟠     |
| `/mining_status`               | ⚫     | `cuprated` does not mine |
| `/out_peers`                   | ⚪     |
| `/pop_blocks`                  | ⚪     |
| `/save_bc`                     | ⚪     |
| `/send_raw_transaction`, `/sendrawtransaction` | 🟣     | `do_not_relay` is incomplete |
| `/set_bootstrap_daemon`        | ⚪     |
| `/set_limit`                   | ⚪     |
| `/set_log_categories`          | ⚪     |
| `/set_log_hash_rate`           | ⚫     | `cuprated` does not mine |
| `/set_log_level`               | ⚪     |
| `/start_mining`                | ⚫     | `cuprated` does not mine |
| `/stop_daemon`                 | 🟠     |
| `/stop_mining`                 | ⚫     | `cuprated` does not mine |
| `/update`                      | ⚫     |

## Binary endpoints
Aliases are listed with their canonical endpoint.

| Endpoint                                      | Status | Notes |
|-----------------------------------------------|--------|-------|
| `/get_blocks.bin`, `/getblocks.bin`           | 🟠     |
| `/get_blocks_by_height.bin`, `/getblocks_by_height.bin` | 🟠     |
| `/get_hashes.bin`, `/gethashes.bin`           | 🟠     |
| `/get_output_distribution.bin`                | 🟠     |
| `/get_o_indexes.bin`                          | 🟠     |
| `/get_outs.bin`                               | 🟠     |
| `/get_transaction_pool_hashes.bin`            | 🟠     |
