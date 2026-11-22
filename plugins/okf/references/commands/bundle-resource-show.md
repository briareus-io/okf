# okf bundle resource show

Read a bounded support file without executing it

```sh
okf bundle resource show .okf-bundle/workflows/check-documentation.md knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | — |
| `<bundle>` | positional | no | — |
| `--max-bytes <value>` | int | no |  (default: `65536`) |
| `--lines <value>` | string | no | — |

Output stream: `bundle-resource`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [bundle-resource](../records/bundle-resource.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--context`, `--guidance`, `--json`, `--revision`, `--selection`, `--workspace-revision`.

Support files are excluded from ordinary knowledge discovery. Explicit inspection is bounded, preserves containment, and never executes scripts or treats support Markdown as concepts.
