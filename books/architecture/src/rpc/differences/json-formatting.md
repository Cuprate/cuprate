# JSON formatting
In general, Cuprate's JSON formatting is very similar to `monerod`, but there are some differences.

This is a list of those differences.

## Pretty vs compact
> TODO: decide when handlers are created if we should allow custom formatting.

Cuprate's RPC (really, [`serde_json`](https://docs.rs/serde_json)) can be configured to use either:
- [Pretty formatting](https://docs.rs/serde_json/latest/serde_json/ser/struct.PrettyFormatter.html)
- [Compact formatting](https://docs.rs/serde_json/latest/serde_json/ser/struct.CompactFormatter.html)

`monerod` uses something _similar_ to pretty formatting.

As an example, pretty formatting:
```json
{
  "number": 1,
  "array": [
    0,
    1
  ],
  "string": "",
  "array_of_objects": [
    {
      "x": 1.0,
      "y": -1.0
    },
    {
      "x": 2.0,
      "y": -2.0
    }
  ]
}
```
compact formatting:
```json
{"number":1,"array":[0,1],"string":"","array_of_objects":[{"x":1.0,"y":-1.0},{"x":2.0,"y":-2.0}]}
```

## Array of objects
`monerod` will format an array of objects like such:
```json
{
  "array_of_objects": [{
    "x": 0.0,
    "y": 0.0,
  },{
    "x": 0.0,
    "y": 0.0,
  },{
    "x": 0.0,
    "y": 0.0
  }]
}
```

Cuprate will format the above like such:
```json
{
  "array_of_objects": [
    {
      "x": 0.0,
      "y": 0.0,
    },
    {
      "x": 0.0,
      "y": 0.0,
    },
    {
      "x": 0.0,
      "y": 0.0
    }
  ]
}
```

## Array of maps containing named objects
An method that contains outputs like this is the `peers` field in the `sync_info` method:
```bash
curl \
    http://127.0.0.1:18081/json_rpc \
    -d '{"jsonrpc":"2.0","id":"0","method":"sync_info"}' \
    -H 'Content-Type: application/json'
```

`monerod` will format an array of maps that contains named objects like such:
```json
{
  "array": [{
    "named_object": {
      "field": ""
    }
  },{
    "named_object": {
      "field": ""
    }
  }]
}
```

Cuprate will format the above like such:
```json
{
  "array": [
    {
      "named_object": {
        "field": ""
      }
    },
    {
      "named_object": {
        "field": ""
      }
    }
  ]
}
```