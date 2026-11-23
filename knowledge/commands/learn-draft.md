---
type: Command
group: meta
mutates: false
title: okf learn draft
description: Create a synthetic Markdown draft with collection on or off.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn draft

Create a synthetic Markdown draft with collection on or off. See the [feedback guide](../guides/capture-local-feedback.md).

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<kind>` | positional | yes | — |
| `--pattern <value>` | string | no | Optional built-in outcome from the local report |

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

Choose a feedback kind even when collection is off. Use invented data and exclude actual identifiers
and usage counts.
