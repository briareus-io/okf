# okf stale

Drift detection: recorded vs. recomputed source fingerprints (+ `stale_after`)

```sh
okf stale knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--fail-on <value>` | string | no | Existing discovery failure threshold; state policy is separate (default: `never`) |
| `--only <value>` | list<string> | no | Display only these source-health states (comma-separated or repeatable) |
| `--summary` | bool | no | Emit a count of examined, displayed, and omitted findings (default: `false`) |
| `--fail-on-state <value>` | list<string> | no | Fail when any examined source has one of these states |

Output stream: `drift,diagnostic-summary`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [drift](../records/drift.md), [diagnostic-summary](../records/diagnostic-summary.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

Sources with fingerprint kinds but no recorded fingerprint are reported as `unrecorded`; unreadable sources are reported even without a baseline. Standard sources without a fingerprint kind are provenance only. Use `--fail-on any` for a failing exit status when findings exist. `validate` checks conformance, not source health.
