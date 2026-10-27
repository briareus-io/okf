---
type: Command
group: check
mutates: false
title: okf validate
description: 'Conformance validation: the spec''s three hard rules only (parseable frontmatter, non-empty type, reserved-filename structure).'
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf validate

Conformance validation: the spec's three hard rules only (parseable frontmatter, non-empty type, reserved-filename structure). Stays permissive about everything else. Exit 1 if nonconformant.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `violation`.

Supported global options: `--bundle-id`, `--json`.
