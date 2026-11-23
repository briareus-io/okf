---
type: Command
group: meta
mutates: false
title: okf learn report
description: Rank affected invocations within each command and CLI version.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn report

Rank affected invocations within each command and CLI version. See the [feedback guide](../guides/capture-local-feedback.md).

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

Each row counts affected invocations within a command family and CLI version. Contention can cause a
missed sample.
