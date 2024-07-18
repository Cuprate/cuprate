# New Transactions

Monero uses the dandelion++ protocol to pass transactions around the network, this flow just describes the actual tx passing between nodes part.

## Flow

This flow is pretty simple, the txs are put into a [new transactions](../levin/protocol.md#notify-new-transactions) notification and sent to
peers.

Hopefully in the future [this is changed](https://github.com/monero-project/monero/issues/9334).

There must be no duplicate txs in the notification.[^duplicate-txs]

---

[^duplicate-txs]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_handler.inl#L991>