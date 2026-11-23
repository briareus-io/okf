---
type: Command
group: meta
mutates: false
title: okf learn purge
description: Delete managed counts and drafts and disable collection.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn purge

Delete managed counts and drafts and disable collection. See the [feedback guide](../guides/capture-local-feedback.md).

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

Purge removes managed drafts and counts. Copies previously exported to user-selected destinations
remain there.
