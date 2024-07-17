# Common P2P Types

This chapter contains definitions of types used in multiple P2P messages.

### Support Flags

Support flags specify any protocol extensions the peer supports, currently only the first bit is used:

`FLUFFY_BLOCKS = 1` - for if the peer supports receiving fluffy blocks.

### Basic Node Data [^b-n-d] { #basic-node-data }

| Fields                 | Type                                  | Description                                                                               |
|------------------------|---------------------------------------|-------------------------------------------------------------------------------------------|
| `network_id`           | A UUID (epee string)                  | A fixed constant value for a specific network (mainnet,testnet,stagenet)                  |
| `my_port`              | u32                                   | The peer's inbound port, if the peer does not want inbound connections this should be `0` |
| `rpc_port`             | u16                                   | The peer's RPC port, if the peer does not want inbound connections this should be `0`     |
| `rpc_credits_per_hash` | u32                                   | States how much it costs to use this node in credits per hashes, `0` being free           |
| `peer_id`              | u64                                   | A fixed ID for the node, set to 1 for anonymity networks                                  |
| `support_flags`        | [support flags](#support-flags) (u32) | Specifies any protocol extensions the peer supports                                       |

### Core Sync Data [^c-s-d] { #core-sync-data }

| Fields                        | Type                   | Description                                                   |
|-------------------------------|------------------------|---------------------------------------------------------------|
| `current_height`              | u64                    | The current chain height                                      |
| `cumulative_difficulty`       | u64                    | The low 64 bits of the cumulative difficulty                  |
| `cumulative_difficulty_top64` | u64                    | The high 64 bits of the cumulative difficulty                 |
| `top_id`                      | [u8; 32] (epee string) | The hash of the top block                                     |
| `top_version`                 | u8                     | The hardfork version of the top block                         |
| `pruning_seed`                | u32                    | THe pruning seed of the node, `0` if the node does no pruning |

### Network Address [^network-addr] { #network-address }

Network addresses are serialized differently than other types, the fields needed depend on the `type` field:

| Fields | Type                                    | Description      |
| ------ | --------------------------------------- | ---------------- |
| `type` | u8                                      | The address type |
| `addr` | An object whose fields depend on `type` | The address      |

#### IPv4

`type = 1`

| Fields   | Type | Description      |
| -------- | ---- | ---------------- |
| `m_ip`   | u32  | The IPv4 address |
| `m_port` | u16  | The port         |


#### IPv6

`type = 2`

| Fields   | Type                   | Description      |
| -------- | ---------------------- | ---------------- |
| `addr`   | [u8; 16] (epee string) | The IPv6 address |
| `m_port` | u16                    | The port         |

#### Tor

TODO:

#### I2p

TODO:

### Peer List Entry Base [^pl-entry-base] { #peer-list-entry-base }

| Fields                 | Type                                | Description                                                                                           |
|------------------------|-------------------------------------|-------------------------------------------------------------------------------------------------------|
| `adr`                  | [Network Address](#network-address) | The address of the peer                                                                               |
| `id`                   | u64                                 | The random, self assigned, ID of this node                                                            |
| `last_seen`            | i64                                 | A field marking when this peer was last seen, although this is zeroed before sending over the network |
| `pruning_seed`         | u32                                 | This peer's pruning seed, `0` if the peer does no pruning                                             |
| `rpc_port`             | u16                                 | This node's RPC port, `0` if this peer has no public RPC port.                                        |
| `rpc_credits_per_hash` | u32                                 | States how much it costs to use this node in credits per hashes, `0` being free                       |

### Tx Blob Entry [^tb-entry] { #tx-blob-entry }

| Fields          | Type                   | Description                             |
| --------------- | ---------------------- | --------------------------------------- |
| `blob`          | bytes (epee string)    | The pruned tx blob                      |
| `prunable_hash` | [u8; 32] (epee string) | The hash of the prunable part of the tx |

### Block Complete Entry [^bc-entry] { #block-complete-entry }

| Fields         | Type                | Description                                               |
|----------------|---------------------|-----------------------------------------------------------|
| `pruned`       | bool                | True if the block is pruned, false otherwise              |
| `block`        | bytes (epee string) | The block blob                                            |
| `block_weight` | u64                 | The block's weight                                        |
| `txs`          | depends on `pruned` | The transaction blobs, the exact type depends on `pruned` |

If `pruned` is true:

`txs` is a vector of [Tx Blob Entry](#tx-blob-entry)

If `pruned` is false:

`txs` is a vector of bytes.

---

[^b-n-d]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L185>

[^c-s-d]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L250>

[^network-addr]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/contrib/epee/include/net/net_utils_base.h#L320>

[^pl-entry-base]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L72>

[^tb-entry]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L121>

[^bc-entry]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L132>
