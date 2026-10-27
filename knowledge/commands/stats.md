---
type: Command
group: check
mutates: false
title: okf stats
description: 'Bundle summary: counts by type, trust-tier distribution, corrected stale count, and orphan count.'
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf stats

Bundle summary: counts by type, trust-tier distribution, corrected stale count, and orphan count.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fail-on <value>` | string | no | Fail on any result: never (default) or any; info/warn/error are compatibility aliases (default: `never`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `stats`.

Supported global options: `--bundle-id`, `--json`.
