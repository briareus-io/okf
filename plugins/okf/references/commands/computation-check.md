# okf computation check

Check and display a computation contract; never executes code

```sh
okf computation check computations/report knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id (leading slash optional), e.g. `tables/customers` |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--details` | bool | no | Compatibility option; detailed output is not implemented for this command (default: `false`) |

Output stream: `computation-contract`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.

Inspect-only: `execution: not-run` is not a passing runtime attestation. Document verification and inspection of executable resources never establish a run verdict.
