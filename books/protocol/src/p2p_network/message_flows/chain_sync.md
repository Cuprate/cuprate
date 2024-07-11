# Chain Sync

Chain sync is the first step in syncing a peers blockchain, it allows a peers to find the split point in their chains and for the peer
to learn about the missing block IDs.

## Flow

The first step is for the initiating peer is to get its compact chain history. The compact chain history must be in reverse chronological
order, with the first block being the top block and the last the genesis, if the only block is the genesis then that only needs to be included
once. The blocks in the middle are not enforced to be at certain locations, however `monerod` will use the top 11 blocks and will then go power
of 2 offsets from then on, i.e 13, 17, 25 ... 

Then, with the compact history, the initiating peer will send a [request chain](../levin/protocol.md#notify-request-chain) message, the receiving
peer will then find the split point and return a [response chain entry](../levin/protocol.md#notify-response-chain-entry) message.

The `response chain entry` will contain a list of block IDs with the first being a common ancestor and the rest being the next blocks that come after
that block in the peers chain.

### Response Checks

- There must be an overlapping block.[^res-overlapping-block]
- The amount of returned block IDs must be less than `25,000`.[^res-max-blocks]

---

[^res-overlapping-block]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_handler.inl#L2568>

[^res-max-blocks]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_handler.inl#L2599>