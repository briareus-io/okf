# okf catalog sync

Materialize exact accepted revisions; --check performs offline verification only

```sh
okf catalog sync --check
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--check` | bool | no |  (default: `false`) |
| `--rebuild` | bool | no |  (default: `false`) |

Output stream: `catalog-sync`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--json`.

Materialize exact accepted revisions only; a missing lock does not authorize resolving a moving branch. --check is offline and cannot prove upstream reachability. Sync does not refresh fingerprints or execute upstream procedures.
