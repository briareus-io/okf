---
type: Command
group: mutate
mutates: true
title: okf artifact put
description: Creates or deliberately replaces an opaque local artifact without refreshing citing concepts.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/artifact.rs
  kind: git-path
- resource: crates/okf-core/src/query/artifact.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf artifact put

Creates a local opaque artifact from authorized bytes. The destination is bundle-relative;
replacement requires the explicit replacement option. Markdown concepts and reserved files
remain protected, and canonical containment prevents writing outside the selected bundle.

Writing bytes does not execute them or refresh their citing concepts. Inspect affected provenance,
reconcile meaning, then deliberately refresh reviewed source baselines through the update workflow.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<resource>` | positional | yes | Destination path relative to the bundle root (opaque or reserved artifacts only) |
| `<input>` | positional | yes | Read artifact bytes from @file (file path is relative to the current directory) |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--create-only` | bool | no | Refuse to overwrite an existing file (the default) (default: `false`) |
| `--replace` | bool | no | Allow replacement of an existing artifact, leaving source fingerprints for drift checks (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `artifact-write`.

Supported global options: `--bundle-id`, `--json`.

Write bytes from `@file` to a bundle-relative opaque or reserved artifact. The default (and `--create-only`) refuses an existing destination; `--replace` permits replacement. Parent directories are created as needed. Concept and configuration files are protected, and paths must remain inside the bundle. Recorded source fingerprints are preserved so `stale` detects changes; review citing concepts before `refresh`.
