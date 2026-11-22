# okf ontology apply

Apply coordinated concept and reusable field-type changes atomically

```sh
okf ontology apply knowledge --from '{field_types: {money: {base: int, min: 0}}}' --dry-run
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--from <value>` | string | yes | Coordinated change document: inline YAML, @file, a file path, or stdin (-) |
| `--dry-run` | bool | no | Validate and preview the change without writing (default: `false`) |
| `--check-dependent-bundles` | bool | no | Examine every registered consumer of a shared ontology sidecar (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.

`--from` accepts a change document (inline or `@ontology-changes.yaml`). Omit the sidecar version header. Supply `field_types` and `concepts` maps, merging named declarations and preserving omitted ones. Top-level `remove` has `field_types` and `concepts` lists; per-concept `remove` has `fields`, `references`, and `relationships` lists. Conflicting operations fail. The complete result validates before a single write. Use `--dry-run` for a preview; null and omission do not request deletion.
