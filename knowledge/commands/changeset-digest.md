---
type: Command
group: query
mutates: false
title: okf changeset digest
description: Reads a guarded baseline token for a selected bundle without an operation document.
sources:
- resource: crates/okf-core/src/mutate/changeset.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/changeset.rs
  kind: git-path
---
# okf changeset digest

Returns the opaque baseline token used by guarded changeset application. It
does not publish authored content. Acquiring a guarded baseline may create
coordination state and requires the corresponding filesystem permissions.
The token covers the inputs declared by the changeset baseline contract; a
later apply must recapture and compare the covered state. It is not a snapshot
for arbitrary follow-up reads.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `baseline`.

Supported global options: `--bundle-id`, `--json`.
