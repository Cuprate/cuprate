# `json-rpc`
JSON-RPC 2.0 types and (de)serialization.

## What
This crate implements the [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification)
for usage in [Cuprate](https://github.com/Cuprate/cuprate).

It contains slight modifications catered towards Cuprate and isn't
necessarily a general purpose implementation of the specification
(see below).

This crate expects you to read the brief JSON-RPC 2.0 specification for context.

## Batching
This crate does not have any types for [JSON-RPC 2.0 batching](https://www.jsonrpc.org/specification#batch).

This is because `monerod` does not support this,
as such, neither does Cuprate.

TODO: citation needed on `monerod` not supporting batching.

## Response changes
[JSON-RPC 2.0's `Response` object](https://www.jsonrpc.org/specification#response_object) usually contains these 2 fields:
- `method`
- `params`

This crate replaces those two with a `body` field that is `#[serde(flatten)]`ed,
and assumes the type within that `body` field is tagged properly, for example:

```rust
# use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use json_rpc::{Id, Request};

// Parameter type.
#[derive(Deserialize, Serialize)]
struct GetBlock {
    height: u64,
}

// Method enum containing all enums.
// All methods are tagged as `method`
// and their inner parameter types are
// tagged with `params` (in snake case).
#[derive(Deserialize, Serialize)]
#[serde(tag = "method", content = "params")] // INVARIANT: these tags are needed
#[serde(rename_all = "snake_case")]          // for proper (de)serialization.
enum Methods {
    GetBlock(GetBlock),
    /* other methods */
}

// Create the request object.
let request = Request::new_with_id(
    Id::Str("hello".into()),
    Methods::GetBlock(GetBlock { height: 123 }),
);

// Serializing properly shows the `method/params` fields
// even though `Request` doesn't contain those fields.
let json = serde_json::to_string_pretty(&request).unwrap();
let expected_json =
r#"{
  "jsonrpc": "2.0",
  "id": "hello",
  "method": "get_block",
  "params": {
    "height": 123
  }
}"#;
assert_eq!(json, expected_json);
```

This is how the method/param types are done in Cuprate.

For reasoning, see: <https://github.com/Cuprate/cuprate/pull/146#issuecomment-2145734838>.