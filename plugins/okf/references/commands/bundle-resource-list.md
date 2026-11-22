# okf bundle resource list

Inventory support files and the manifest without loading instruction bodies

```sh
okf bundle resource list knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `bundle-resource`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [bundle-resource](../records/bundle-resource.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--bundle-revision`, `--catalog`, `--context`, `--guidance`, `--json`, `--revision`, `--selection`, `--workspace-revision`.

Support files are excluded from ordinary knowledge discovery. Explicit inspection is bounded, preserves containment, and never executes scripts or treats support Markdown as concepts.
