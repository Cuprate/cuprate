# `cuprate-rpc-interface`
This crate provides Cuprate's RPC _interface_.

```text
            cuprate-rpc-interface provides these parts
                                 │
                            ┌────┴────┐
┌───────────────────────────┤         ├───────────────────┐
▼                           ▼         ▼                   ▼
CLIENT -> ROUTE -> REQUEST -> HANDLER -> RESPONSE -> CLIENT
                             ▲       ▲
                             └───┬───┘
                                 │
                      You provide this part
```

Everything coming _in_ from a client including:
- Any lower-level HTTP stuff
- Endpoint routing
- Request (de)serialization

is handled by this crate.

This is where your [`RpcHandler`] turns this [`Request`] into a [`Response`].

You hand this `Response` back to `cuprate-rpc-interface` and it will take care of sending it back to the client.

The main handler used by Cuprate is implemented in the `cuprate-rpc-handler` crate;
it implements the regular RPC server modeled after `monerod`.

# Router

# Requests, responses, and errors

# The RPC handler [`Service`](tower::Service)

# Routes

# Feature flags
List of feature flags for `cuprate-rpc-interface`.

All are enabled by default.

| Feature flag | Does what |
|--------------|-----------|