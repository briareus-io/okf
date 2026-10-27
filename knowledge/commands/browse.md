---
type: Command
group: query
mutates: false
title: okf browse
description: Reads a directory's index.md for progressive disclosure, synthesizing it when absent.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/query.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf browse

Reads a directory's checked-in `index.md` for specification-native progressive disclosure. If the file is absent, synthesizes the same directory listing in memory from bundle concepts without writing it.

Use the root view first, then pass `--directory <path>` to descend into a promising branch. JSON output identifies whether the content came from a file or was synthesized.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--directory <value>` | string | no | Bundle-relative directory to browse (default: root `/`) (default: `/`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `index`.

Supported global options: `--bundle-id`, `--json`.
