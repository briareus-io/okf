---
type: Command
group: meta
mutates: true
title: okf catalog update
description: Plans and explicitly accepts reviewed remote dependency revisions.
---
# okf catalog update

Plans and explicitly accepts reviewed remote dependency revisions.

See the [bundle customization guide](../guides/customize-a-bundle.md) and
[remote dependency guide](../guides/remote-dependencies.md) for selection, guidance, and access boundaries.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--plan <value>` | string | no | — |
| `--apply <value>` | string | no | — |
| `--allow-incomplete-impact` | bool | no |  (default: `false`) |

Global `--json` requests NDJSON.

Output stream: `catalog-update-plan,catalog-update-applied,plan-file`.

Supported global options: `--bundle-id`, `--catalog`, `--catalog-scope`, `--json`, `--scope-bundle`.

Planning obtains and reports an exact candidate without accepting it. Apply rechecks plan inputs and accepts that reviewed commit without re-resolving the upstream ref. Dependency update scope and consumer impact scope are distinct. Incomplete-impact acceptance never overrides compatibility, integrity, or stale-input failures.
