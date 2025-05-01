# RPC
`monerod`'s daemon RPC has 3 kinds of interfaces:
1. JSON-RPC 2.0 methods called at the `/json_rpc` endpoint, e.g. [`get_block`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block)
1. JSON endpoints, e.g. [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height)
1. Binary endpoints, e.g. [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin)

`cuprated`'s RPC aims to mirror `monerod`'s as much as it can. The end-goal is compatability with common use-cases such as wallet software.

This section contains the status of endpoints/methods in `cuprated`.

| Status | Meaning |
|--------|---------|
| 🟢     | Enabled and tested
| 🟠     | Enabled but not tested
| ⚪     | Not enabled yet
| ⚫     | Not planned to be supported

## JSON-RPC methods
| Method                         | Status | Notes   |
|--------------------------------|--------|---------|
| `get_block_count`              | ⚪     |
| `get_last_block_header`        | ⚪     |
| `get_block_header_by_hash`     | ⚪     |
| `get_block_header_by_height`   | ⚪     |
| `get_block`                    | ⚪     |
| `hard_fork_info`               | ⚪     |
| `on_get_block_hash`            | ⚪     |
| `get_block_headers_range`      | ⚪     |
| `get_connections`              | ⚪     |
| `set_bans`                     | ⚪     |
| `get_bans`                     | ⚪     |
| `banned`                       | ⚪     |
| `get_version`                  | ⚪     |
| `get_output_histogram`         | ⚪     |
| `get_fee_estimate`             | ⚪     |
| `calc_pow`                     | ⚪     |
| `flush_transaction_pool`       | ⚪     |
| `relay_tx`                     | ⚪     |
| `get_coinbase_tx_sum`          | ⚪     |
| `get_alternate_chains`         | ⚪     |
| `sync_info`                    | ⚪     |
| `get_miner_data`               | ⚪     |
| `submit_block`                 | ⚪     |
| `get_info`                     | ⚪     |
| `generate_blocks`              | ⚪     |
| `add_aux_pow`                  | ⚪     |
| `get_transaction_pool_backlog` | ⚪     |
| `get_output_distribution`      | ⚪     |
| `get_tx_ids_loose`             | ⚪     | Not implemented in `monerod` release branch yet
| `flush_cache`                  | ⚫     | `cuprated` does not require this
| `prune_blockchain`             | ⚫     |

## JSON endpoints
| Endpoint                       | Status | Notes   |
|--------------------------------|--------|---------|
| `/get_height`                  | ⚪     |
| `/get_outs`                    | ⚪     |
| `/is_key_image_spent`          | ⚪     |
| `/get_transaction_pool_hashes` | ⚪     |
| `/get_transaction_pool`        | ⚪     |
| `/get_transaction_pool_stats`  | ⚪     |
| `/save_bc`                     | ⚪     |
| `/stop_daemon`                 | ⚪     |
| `/pop_blocks`                  | ⚪     |
| `/get_peer_list`               | ⚪     |
| `/get_public_nodes`            | ⚪     |
| `/get_alt_blocks_hashes`       | ⚪     |
| `/send_raw_transaction`        | ⚪     |
| `/get_transactions`            | ⚪     |
| `/get_limit`                   | ⚪     |
| `/set_limit`                   | ⚪     |
| `/out_peers`                   | ⚪     |
| `/in_peers`                    | ⚪     |
| `/get_net_stats`               | ⚪     |
| `/set_log_level`               | ⚪     | Will use `tracing` levels
| `/set_log_categories`          | ⚪     | Could be re-purposed to use `tracing` filters
| `/set_bootstrap_daemon`        | ⚪     | Requires bootstrap implementation
| `/start_mining`                | ⚫     | `cuprated` does not mine
| `/stop_mining`                 | ⚫     | `cuprated` does not mine
| `/mining_status`               | ⚫     | `cuprated` does not mine
| `/set_log_hash_rate`           | ⚫     | `cuprated` does not mine
| `/update`                      | ⚫     |

## Binary endpoints
| Endpoint                           | Status | Notes   |
|------------------------------------|--------|---------|
| `/get_blocks_by_height.bin`        | ⚫     |
| `/get_hashes.bin`                  | ⚫     |
| `/get_output_indexes.bin`          | ⚫     |
| `/get_outs.bin`                    | ⚫     |
| `/get_blocks.bin`                  | ⚫     |
| `/get_transaction_pool_hashes.bin` | ⚫     |
| `/get_output_distribution.bin`     | ⚫     |
