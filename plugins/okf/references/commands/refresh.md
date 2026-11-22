# okf refresh

Re-record source fingerprints after a change is acknowledged

```sh
okf refresh policies/review knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id to refresh |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fail-on <value>` | string | no | Fail (exit 1) when any source is skipped: never (default) \| skipped \| any (default: `never`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Output stream: `change`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

Refresh updates supported source fingerprints across the concept, not content or standard source modification dates. Review those sources against the final content first, whether or not the content needed rewriting. JSON reports `updated`/`unchanged` counts and `skipped` entries with reasons. Missing, unavailable, unsupported, or malformed required sources block the entire refresh before publication. `--fail-on skipped` rejects provenance-only skips before publication as well. Report unresolved sources instead of treating refresh as proof of complete synchronization.
