# Environment variables

In general, environment variables will override both [config](./config.md) and [CLI](./cli.md) values.

<!--
TODO: flags for:
- randomx
- tracing
- rayon
- tokio
- dirs
-->

| Environment variable      | Type | Description    |
|---------------------------|------|----------------|
| `$HOME`, `$XDG_DATA_HOME` | Path | `cuprated` will create files/directories within this directory.