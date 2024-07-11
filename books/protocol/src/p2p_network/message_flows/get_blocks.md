# Get Blocks

The get block flow is used to download batches of blocks from a peer.

## Flow

The initiating peer needs a list of block IDs that the receiving peer has, this can be done with
the [chain sync flow](./chain_sync.md).

With a list a block IDs the initiating peer will send a [get objects request](../levin/protocol.md#notify-request-get-objects) message, the receiving
peer will then respond with [get objects response](../levin/protocol.md#notify-response-get-objects).

### Request Checks

- The amount of blocks must be less than `100`.[^max-block-requests] 

---

[^max-block-requests]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_handler.inl#L1089>
