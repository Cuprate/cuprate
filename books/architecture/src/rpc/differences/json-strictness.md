# JSON strictness
This is a list of behavior that `monerod`'s JSON parser allows, that Cuprate's JSON parser ([`serde_json`](https://docs.rs/serde_json)) does not.

In general, `monerod`'s parser is quite lenient, allowing invalid JSON in many cases.
Cuprate's (really, `serde_json`) JSON parser is quite strict, essentially sticking to
the [JSON specification](https://datatracker.ietf.org/doc/html/rfc8259).

Cuprate also makes some decisions that are _different_ than `monerod`, but are not necessarily more or less strict.

## Missing closing bracket
`monerod` will accept JSON missing a final closing `}`.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":"0","method":"get_block_count"' \
	-H 'Content-Type: application/json'
```

## Trailing ending comma
`monerod` will accept JSON containing a final trailing `,`.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":"0","method":"get_block_count",}' \
	-H 'Content-Type: application/json'
```

## Allowing `-` in fields
`monerod` allows `-` as a valid value in certain fields, **not a string `"-"`, but the character `-`**.

The fields where this is allowed seems to be any field `monerod` does not explicitly look for, examples include:
- `jsonrpc`
- `id`
- `params` (where parameters are not expected)
- Any ignored field

The [JSON-RPC 2.0 specification does state that the response `id` should be `null` upon errors in detecting the request `id`](https://wwwjsonrpc.org/specification#response_object), although in this case, this is invalid JSON and should not make it this far. The response will contain the default `id: 0` in this case.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":-,"id":-,"params":-,"IGNORED_FIELD":-,"method":"get_block_count"}' \
	-H 'Content-Type: application/json'
```