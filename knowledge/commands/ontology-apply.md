---
type: Command
group: mutate
mutates: true
title: okf ontology apply
description: Applies dependent ontology declarations and explicit removals in one validated write.
sources:
- resource: crates/okf-core/src/ontology/edit.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
last_modified: 2026-10-05T10:16:06Z
---
# okf ontology apply

Applies dependent ontology declarations and explicit removals in one validated write.

Input `--from` accepts inline YAML/JSON, `@file`, stdin `-`, or a definition file path. The change
envelope has `field_types` and `concepts` maps, preserving omitted named declarations. Top-level
`remove` lists `field_types` and `concepts`; per-concept `remove` lists `fields`, `references`, and
`relationships`. Supplied declarations replace completely, and conflicting operations fail.
The final ontology validates before one sidecar publication. `--dry-run`
previews without publishing the sidecar; coordination state may be created.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--from <value>` | string | yes | Coordinated change document: inline YAML, @file, a file path, or stdin (-) |
| `--dry-run` | bool | no | Validate and preview the change without writing (default: `false`) |
| `--check-dependent-bundles` | bool | no | Examine every registered consumer of a shared ontology sidecar (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.

Supported global options: `--bundle-id`, `--json`.

`--from` accepts `field_types` and `concepts` maps, merging named declarations and preserving omitted ones. Top-level `remove` has `field_types` and `concepts` lists; per-concept `remove` has `fields`, `references`, and `relationships` lists. Conflicting operations fail. The complete result validates before a single write. Use `--dry-run` for a preview; null and omission do not request deletion.
