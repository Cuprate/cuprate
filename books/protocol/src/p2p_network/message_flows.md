# Message Flows

Message flows are sets of messages sent between peers, that achieve an identifiable goal, like a handshake.
Some message flows are complex, involving many message types, whereas others are simple, requiring only 1.

The message flows here are not every possible request/response.

When documenting checks on the messages, checks not on the message will not be included. For example when receiving
a handshake monerod will check if we have too many incoming connections, this check would not be included in the
checks on the handshake request.

## Different Flows

- [Handshakes](./message_flows/handshake.md)
- [Timed Sync](./message_flows/timed_sync.md)
- [New Block](./message_flows/new_block.md)
- [New Transactions](./message_flows/new_transactions.md)
- [Chain Sync](./message_flows/chain_sync.md)

