# okf validate

Conformance validation — the spec's three hard rules only

```sh
okf validate knowledge --profile bundle
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | — |
| `--profile <value>` | string | no | Core preserves OKF v0.2 file boundaries; bundle excludes the support namespace (default: `core`) |

Output stream: `violation,validation-profile`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [validation-profile](../records/validation-profile.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.


