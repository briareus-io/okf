---
type: Command
group: mutate
mutates: true
title: okf changeset apply
description: Publish validated coordinated record changes with guarded inputs and recoverable per-file writes.
sources:
- resource: crates/okf-core/src/mutate/changeset.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
---
# okf changeset apply

Publish validated coordinated record changes with guarded inputs and recoverable per-file writes.

See the [complete CLI reference](index.md) for operation schemas and guarantees.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--from <value>` | string | yes | Version 1 change document: inline YAML/JSON, @file, or stdin (-) |
| `--expect <value>` | string | no | Require this base digest from a previous plan before applying |
| `--dry-run` | bool | no | Validate and preview without publishing authored files (also the behavior of changeset plan) (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `changeset-file,changeset-summary`.

Supported global options: `--bundle-id`, `--json`.

Supply a version 1 YAML/JSON document with an ordered `operations` list. Supported operations: `create`/`replace` with `path` and complete Markdown `content`; `edit` with `concept`, optional frontmatter RFC 6902 `patch` and optional `body`; `move` with `from`/`to` concept IDs; `retarget` with `from`/`to` and optional `within` concept-ID list; `remove` with an exact bundle-relative file `path`; `put-artifact` with `path`, text `content`, and optional `replace: true`. Splits and merges combine creates, explicit retarget mappings, and removal; no semantic mapping is inferred. Operations run on an isolated copy; the final bundle must be conformant and introduce no dangling supported local references. Checks cover local Markdown/artifact links and file, line-range, and markdown-heading sources; Git, URL, and producer-specific sources are not checked. Existing unrelated broken references may remain. Concept replacement/edit invalidates verification; content cannot supply verification. Plan and `apply --dry-run` print all file diffs plus a base digest without publishing authored bundle content; coordination state may be created. Use `apply --expect <base-digest>` to guard a reviewed baseline, with the same reviewed change document. Publication is journaled and atomic per file, not an atomic snapshot for concurrent readers. Cooperating CLI writers refuse a pending transaction. Cross-bundle writes and configuration changes are unsupported. On interrupted publication use `changeset recover`; do not remove the journal manually.
