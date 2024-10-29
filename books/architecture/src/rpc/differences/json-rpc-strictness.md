# JSON-RPC strictness
This is a list of behavior that `monerod`'s JSON-RPC implementation allows, that Cuprate's JSON-RPC implementation does not.

In general, `monerod`'s JSON-RPC is quite lenient, going against the specification in many cases.
Cuprate's JSON-RPC implementation is slightly more strict.

Cuprate also makes some decisions that are _different_ than `monerod`, but are not necessarily more or less strict.

## Allowing an incorrect `jsonrpc` field
[The JSON-RPC 2.0 specification states that the `jsonrpc` field must be exactly `"2.0"`](https://www.jsonrpc.org/specification#request_object).

`monerod` allows `jsonrpc` to:
- Be any string
- Be an empty array
- Be `null`
- Not exist at all

Examples:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"???","method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":[],"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":null,"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

## Allowing `id` to be any type
JSON-RPC 2.0 responses must contain the same `id` as the original request.

However, the [specification states](https://www.jsonrpc.org/specification#request_object):

> An identifier established by the Client that MUST contain a String, Number, or NULL value if included

`monerod` does not check this and allows `id` to be any JSON type, for example, a map:
```bash
curl \
    http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":{"THIS":{"IS":"ALLOWED"}},"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```

The response:
```json
{
  "id": {
    "THIS": {
      "IS": "ALLOWED"
    }
  },
  "jsonrpc": "2.0",
  "result": {
    "count": 3210225,
    "status": "OK",
    "untrusted": false
  }
}
```

## Responding with `id:0` on error
The JSON-RPC [specification states](https://www.jsonrpc.org/specification#response_object):

> If there was an error in detecting the id in the Request object (e.g. Parse error/Invalid Request), it MUST be Null.

Although, `monerod` will respond with `id:0` in these cases.

```bash
curl \
    http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":asdf,"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```
Response:
```bash
{
  "error": {
    "code": -32700,
    "message": "Parse error"
  },
  "id": 0,
  "jsonrpc": "2.0"
}
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