---
type: Command
group: query
mutates: false
title: okf bundle resource list
description: Lists support resources explicitly without exposing them as knowledge concepts.
---
# okf bundle resource list

Lists support resources explicitly without exposing them as knowledge concepts.

See the [bundle customization guide](../guides/customize-a-bundle.md) and
[remote dependency guide](../guides/remote-dependencies.md) for selection, guidance, and access boundaries.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `bundle-resource`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--guidance`, `--json`, `--revision`, `--workspace-revision`.

Support files are excluded from ordinary knowledge discovery. Explicit inspection is bounded, preserves containment, and never executes scripts or treats support Markdown as concepts.
