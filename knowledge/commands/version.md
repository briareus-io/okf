---
type: Command
group: meta
mutates: false
title: okf version
description: Prints the CLI version and the OKF spec version(s) it supports.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf version

Prints the CLI version and the OKF spec version(s) it supports.

## Arguments

_No arguments._

Global `--json` requests NDJSON.

Output stream: `version`.

Supported global options: `--json`.
