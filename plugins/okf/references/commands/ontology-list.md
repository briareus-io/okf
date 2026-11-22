# okf ontology list

List the defined concept types

```sh
okf ontology list knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `ontology_type`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--context`, `--guidance`, `--json`, `--revision`, `--selection`, `--workspace-revision`.


