---
type: Command
group: meta
mutates: false
title: okf learn disable
description: Stop counting and retain managed feedback.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn disable

Stop counting and retain managed feedback. See the [feedback guide](../guides/capture-local-feedback.md).

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

Disabling collection preserves existing aggregates and drafts.
