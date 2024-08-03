# JSON strictness
This is a list of behavior that `monerod`'s JSON parser allows, that Cuprate's JSON parser ([`serde_json`](https://docs.rs/serde_json)) does not.

In general, `monerod`'s parser is quite leniant, allowing invalid JSON in many cases.
Cuprate's (really, `serde_json`) JSON parser is quite strict, essentially sticking to
the JSON specification.

Cuprate also makes some decisions that are _different_ than `monerod`, but are not necessarily more or less strict.

## Missing closing bracket
`monerod` will accept JSON missing a closing `}`.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":"0","method":"get_block_count"' \
	-H 'Content-Type: application/json'
```

## Extra fields
`monerod` will ignore extra fields within JSON from the user.

Cuprate will return an error if any unknown field is present.

Example:
```bash
curl \
	http://127.0.0.1:18081/json_rpc \
	-d '{"jsonrpc":"2.0","id":"0","method":"get_block_count","IGNORED_FIELD":0}' \
	-H 'Content-Type: application/json'
```