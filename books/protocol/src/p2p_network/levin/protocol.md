# Protocol Messages

This chapter describes protocol messages, and documents the current protocol messages.

## Levin

All protocol messages are in the notification levin format. Although there are some messages that fall under requests/responses
levin will treat them as notifications


This means requests will NOT set the [expect response bit](./levin.md#expect-response) and responses will set the return code to [`0`](./levin.md#return-code).

## Messages

### Notify New Block

ID: `2001`[^notify-new-block-id]

| Fields                      | Type                                                            | Description              |
| --------------------------- | --------------------------------------------------------------- | ------------------------ |
| `b`                         | [Block Complete Entry](../common_types.md#block-complete-entry) | The full block           |
| `current_blockchain_height` | u64                                                             | The current chain height |

### Notify New Transactions

ID: `2002`[^notify-new-transactions-id]

| Fields              | Type              | Description                                            |
| ------------------- | ----------------- | ------------------------------------------------------ |
| `txs`               | A vector of bytes | The txs                                                |
| `_`                 | Bytes             | Padding to prevent traffic volume analysis             |
| `dandelionpp_fluff` | bool              | True if this message contains fluff txs, false if stem |

### Notify Request Get Objects

ID: `2003`[^notify-request-get-objects-id]

| Fields   | Type                                               | Description                                     |
| -------- | -------------------------------------------------- | ----------------------------------------------- |
| `blocks` | A vector of [u8; 32] serialized as a single string | The blocks IDs requested                        |
| `prune`  | bool                                               | A bool for if we want the blocks in pruned form |

### Notify Response Get Objects

ID: `2004`[^notify-response-get-objects-id]

| Fields                      | Type                                                                        | Description                    |
| --------------------------- | --------------------------------------------------------------------------- | ------------------------------ |
| `blocks`                    | A vector of [Block Complete Entry](../common_types.md#block-complete-entry) | The blocks that were requested |
| `missed_ids`                | A vector of [u8; 32] serialized as a single string                          | IDs of any missed blocks       |
| `current_blockchain_height` | u64                                                                         | The current blockchain height  |

### Notify Request Chain

ID: `2006`[^notify-request-chain-id]

| Fields      | Type                                               | Description                                                                                           |
| ----------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `block_ids` | A vector of [u8; 32] serialized as a single string | A list of block ids in reverse chronological order, the top and genesis block will always be included |
| `prune`     | bool                                               | For if we want the response to contain pruned blocks                                                  |

### Notify Response Chain Entry

ID: `2007`[^notify-response-chain-entry-id]

| Fields                        | Type                                               | Description                                    |
| ----------------------------- | -------------------------------------------------- | ---------------------------------------------- |
| `start_height`                | u64                                                | The start height of the entry                  |
| `total_height`                | u64                                                | The height of the peers blockchain             |
| `cumulative_difficulty`       | u64                                                | The low 64 bits of the cumulative difficulty   |
| `cumulative_difficulty_top64` | u64                                                | The high 64 bits of the cumulative difficulty  |
| `m_block_ids`                 | A vector of [u8; 32] serialized as a single string | The blocks IDs in this entry                   |
| `m_block_weights`             | A vector of u64 serialized as a single string      | The blocks weights                             |
| `first_block`                 | bytes (epee string)                                | The header of the first block in `m_block_ids` |

### Notify New Fluffy Block

ID: `2008`[^notify-new-fluffy-block-id]

| Fields                      | Type                                                            | Description                           |
| --------------------------- | --------------------------------------------------------------- | ------------------------------------- |
| `b`                         | [Block Complete Entry](../common_types.md#block-complete-entry) | The block, may or may not contain txs |
| `current_blockchain_height` | u64                                                             | The current chain height              |

### Notify Request Fluffy Missing Tx

ID: `2009`[^notify-request-fluffy-missing-tx-id]

| Fields                      | Type                                          | Description                                |
| --------------------------- | --------------------------------------------- | ------------------------------------------ |
| `block_hash`                | [u8; 32] serialized as a string               | The block hash txs are needed from         |
| `current_blockchain_height` | u64                                           | The current chain height                   |
| `missing_tx_indices`        | A vector of u64 serialized as a single string | The indexes of the needed txs in the block |

### Notify Get Txpool Compliment

ID: `2010`[^notify-get-txpool-compliment-id]

| Fields   | Type                                        | Description            |
| -------- | ------------------------------------------- | ---------------------- |
| `hashes` | A vector of [u8; 32] serialized as a string | The current txpool txs |

---

[^notify-new-block-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L174>

[^notify-new-transactions-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L194>

[^notify-request-get-objects-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L215>

[^notify-response-get-objects-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L232>

[^notify-request-chain-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L274>

[^notify-response-chain-entry-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L291>

[^notify-new-fluffy-block-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L324>

[^notify-request-fluffy-missing-tx-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L344>

[^notify-get-txpool-compliment-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L366>
