# New Block

This is used whenever a new block is to be sent to peers. Only the fluffy block flow is described here, as the other method is deprecated.

## Flow

First the peer with the new block will send a [new fluffy block](../levin/protocol.md#notify-new-fluffy-block) notification, if the receiving
peer has all the txs in the block then the flow is complete. Otherwise the peer sends a [fluffy missing transactions request](../levin/protocol.md#notify-request-fluffy-missing-tx) 
to the first peer, the first peer will then respond with again a [new fluffy block](../levin/protocol.md#notify-new-fluffy-block) notification but
with the transactions requested.  

```bob
                          
     ,-----------.         ,----------.  
     | Initiator |         | Receiver |
     `-----+-----'         `-----+----'
           |  New Fluffy Block   |
           |-------------------->|
           |                     |
           | Missing Txs Request |
           |<- - - - - - - - - - |
           |                     |
           |  New Fluffy Block   |
           | - - - - - - - - - ->|
           |                     |
           |                     |
           V                     v
```

