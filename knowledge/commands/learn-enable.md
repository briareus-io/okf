---
type: Command
group: meta
mutates: false
title: okf learn enable
description: Enable fixed-outcome counting after interactive consent.
sources:
- resource: crates/okf-cli/src/learn.rs
  kind: git-path
---
# okf learn enable

Enable fixed-outcome counting after interactive consent. See the [local feedback capability](../contributing/design/local-feedback.md).

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `feedback`.

Supported global options: `--json`.

Type enable at the terminal prompt to begin counting qualifying outcomes across checkouts.
