---
type: Command
group: check
mutates: false
title: okf scan
description: Walks the physical bundle tree and reports every Markdown file without parsing concepts.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf scan

Walks the physical bundle tree and reports every Markdown file, including reserved and Git-ignored files, without parsing concepts. Use [okf source-scan](source-scan.md) for a general repository inventory.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fail-on <value>` | string | no | Fail on any result: never (default) or any; info/warn/error are compatibility aliases (default: `never`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `scan`.

Supported global options: `--bundle-id`, `--json`.
