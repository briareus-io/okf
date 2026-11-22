# okf catalog update

Plan an upstream update, or explicitly apply a saved reviewed plan

```sh
okf catalog update --plan update-plan.json
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--plan <value>` | string | no | — |
| `--apply <value>` | string | no | — |
| `--allow-incomplete-impact` | bool | no |  (default: `false`) |

Output stream: `catalog-update-plan,catalog-update-applied,plan-file`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [catalog-update-plan](../records/catalog-update-plan.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--json`, `--scope-bundle`.

Planning obtains and reports an exact candidate without accepting it. Apply rechecks plan inputs and accepts that reviewed commit without re-resolving the upstream ref. Dependency update scope and consumer impact scope are distinct. Incomplete-impact acceptance never overrides compatibility, integrity, or stale-input failures.
