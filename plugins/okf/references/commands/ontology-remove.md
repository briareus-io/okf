# okf ontology remove

Remove a concept type

```sh
okf ontology remove Invoice knowledge --dry-run
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Name of the definition to remove |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--dry-run` | bool | no | Validate and preview the change without writing (default: `false`) |
| `--check-dependent-bundles` | bool | no | Examine every registered consumer of a shared ontology sidecar (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.


