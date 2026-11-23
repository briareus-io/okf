---
type: Command
group: meta
mutates: false
title: okf learn export
description: Preview a local destination and draft, then confirm the exact copy.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn export

Preview a local destination and draft, then confirm the exact copy. See the [feedback guide](../guides/capture-local-feedback.md).

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<draft>` | positional | yes | Managed draft filename returned by learn draft |
| `--output <value>` | string | yes | Explicit new file to create |
| `--confirm <value>` | string | no | Digest from the reviewed preview, binding content and destination |

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

Preview the complete text and destination first. Pass the returned digest only after review. The
destination must be a new file.
