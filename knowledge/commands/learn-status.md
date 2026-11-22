---
type: Command
group: meta
mutates: false
title: okf learn status
description: Inspect the private state location, consent, draft count, and data size.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn status

Inspect the private state location, consent, draft count, and data size. See the [local feedback capability](../contributing/design/local-feedback.md).

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

The directory is per OS user. Status works before the first draft or opt-in.
