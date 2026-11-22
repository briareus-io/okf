# okf ontology field-type show

Show a reusable field-type definition and its effective fields

```sh
okf ontology field-type show money knowledge --json
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Name of the definition to inspect |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `ontology_field_type`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [ontology_field_type](../records/ontology_field_type.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--context`, `--guidance`, `--json`, `--revision`, `--selection`, `--workspace-revision`.

Inspect authored reusable definitions and resolved inherited constraints. JSON includes both definition and effective fields.
