---
type: Command
group: check
mutates: false
title: okf source-scan
description: Inventories every regular file in a source directory without parsing it as an OKF bundle.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf source-scan

Maps a repository or directory for ingest workflows. Unlike [okf scan](scan.md), it includes non-Markdown source artifacts and performs no concept parsing.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<directory>` | positional | yes | Source directory to inventory; it need not be an OKF bundle |

Global `--json` requests NDJSON.

Output stream: `source-file`.

Supported global options: `--json`.
