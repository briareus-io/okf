# okf format

Format YAML and opt-in Markdown prose without changing verification

```sh
okf format --bundle knowledge --check
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | no | One concept ID; omit to format the entire bundle |
| `--bundle <value>` | path | no | Bundle directory (otherwise use the current context) |
| `--check` | bool | no | Report drift without writing; exit 1 when formatting is needed (default: `false`) |
| `--dry-run` | bool | no | Preview the complete diff without writing (default: `false`) |

Output stream: `format-file,format-summary`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [format-file](../records/format-file.md), [format-summary](../records/format-summary.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`, `--selection`.

Presentation-only formatting preserves verification and generation metadata. `--check` does not publish and exits 1 for formatting drift; `--dry-run` previews the diff. Omit the concept ID for the whole selected bundle. YAML uses `settings.formatting.yaml` (readable by default, target width 100; legacy retains the previous serializer). Markdown wrapping is disabled by default and runs only here when `settings.formatting.markdown.wrap_prose` is true. A width of 0 disables wrapping. Unsupported Markdown structures remain unchanged. YAML comments and scalar styles may normalize; file fingerprints may change. Formatting is not conformance repair, fingerprint refresh, or claim verification.
