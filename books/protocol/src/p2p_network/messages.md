# P2P Messages

This chapter contains every P2P message.

## Index

## Types

Types used in multiple P2P messages.

### Support Flags

Support flags specify any protocol extensions the peer supports, currently only the first bit is used:

`FLUFFY_BLOCKS = 1` - for if the peer supports receiving fluffy blocks.

### Basic Node Data

| Fields                 | Type (Epee Type)                      | Description                                                                              |
| ---------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------- |
| `network_id`           | A UUID (String)                       | A fixed constant value for a specific network (mainnet,testnet,stagenet)                  |
| `my_port`              | u32 (u32)                             | The peer's inbound port, if the peer does not want inbound connections this should be `0` |
| `rpc_port`             | u16 (u16)                             | The peer's RPC port, if the peer does not want inbound connections this should be `0`     |
| `rpc_credits_per_hash` | u32 (u32)                             | TODO                                                                                     |
| `peer_id`              | u64 (u64)                             | A fixed ID for the node, set to 1 for anonymity networks                                 |
| `support_flags`        | [support flags](#support-flags) (u32) | Specifies any protocol extensions the peer supports                                      |

## Messages

### Handshake Requests

levin command: 1001

| Fields      | Type (Epee Type)                             | Description |
| ----------- | -------------------------------------------- | ----------- |
| `node_data` | [basic node data](#basic-node-data) (Object) |             |
|             |                                              |             |
