---
type: Command
group: meta
mutates: true
title: okf catalog sync
description: Materializes exact accepted Git revisions or checks local lock correspondence offline.
---
# okf catalog sync

Materializes exact accepted Git revisions or checks local lock correspondence offline.

See the [bundle customization guide](../guides/customize-a-bundle.md) and
[remote dependency guide](../guides/remote-dependencies.md) for selection, guidance, and access boundaries.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--check` | bool | no |  (default: `false`) |
| `--rebuild` | bool | no |  (default: `false`) |

Global `--json` requests NDJSON.

Output stream: `catalog-sync`.

Supported global options: `--bundle-id`, `--catalog`, `--json`.

Materialize exact accepted revisions only; a missing lock does not authorize resolving a moving branch. --check is offline and cannot prove upstream reachability. Sync does not refresh fingerprints or execute upstream procedures.
