# JSON-RPC strictness
This is a list of behavior that `monerod`'s JSON-RPC implementation allows, that Cuprate's JSON-RPC implementation does not.

In general, `monerod`'s JSON-RPC is quite lenient, going against the specification in many cases.
Cuprate's JSON-RPC implementation is slightly more strict.

Cuprate also makes some decisions that are _different_ than `monerod`, but are not necessarily more or less strict.

## Allowing anything in the `jsonrpc` field
[The JSON-RPC 2.0 specification states that the `jsonrpc` field must be exactly `"2.0"`](https://www.jsonrpc.org/specification#request_object).

`monerod` allows:
- The field to be any string
- The field to be any type
- The field to not even exist at all

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

## Allowing `-` in the `id` field
`monerod` allows `-` to be in the `id` field, **not a string `"-"`, but just the character `-`**.

The [JSON-RPC 2.0 specification does state that the response `id` should be `null` upon errors in detecting the request `id`](https://www.jsonrpc.org/specification#response_object), although in this case, this is invalid JSON and should not make it this far.

The response also contains `id: 0` instead.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":------,"method":"get_block_count"}' \
	-H 'Content-Type: application/json'

```

## Responding to notifications
> TODO: decide on Cuprate behavior <https://github.com/Cuprate/cuprate/pull/233#discussion_r1704611186>

Requests that have no `id` field are "notifications".

[The JSON-RPC 2.0 specification states that requests without
an `id` field must _not_ be responded to](https://www.jsonrpc.org/specification#notification).

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

## Upper/mixed case fields
`monerod` will accept upper/mixed case fields on:
- `jsonrpc`
- `id`

`method` however, is checked.

The JSON-RPC 2.0 specification does not outright state what case to support,
although, Cuprate only supports lowercase as supporting upper/mixed case
is more code to add as `serde` by default is case-sensitive on `struct` fields.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsONrPc":"2.0","iD":0,"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```