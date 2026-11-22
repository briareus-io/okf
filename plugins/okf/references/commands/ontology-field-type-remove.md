# okf ontology field-type remove

Remove a reusable field type if the resulting ontology remains valid

```sh
okf ontology field-type remove money knowledge --dry-run
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Reusable field-type name to remove |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--dry-run` | bool | no | Validate and preview the change without writing (default: `false`) |
| `--check-dependent-bundles` | bool | no | Examine every registered consumer of a shared ontology sidecar (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.

Reusable field types are ontology-wide definitions. Add/update `--from` accepts a whole definition; update replaces it completely. Inspect consumers before removal or tightening. `--dry-run` validates the proposed ontology without publishing the sidecar; coordination state may be created.
