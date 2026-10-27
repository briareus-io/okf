---
type: Command
group: mutate
mutates: true
title: okf ontology field-type remove
description: Removes a reusable structured field type from the ontology.
sources:
- resource: crates/okf-core/src/ontology/edit.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
last_modified: 2026-10-05T10:16:06Z
---
# okf ontology field-type remove

Removes a reusable structured field type from the ontology.

Removal validates the resulting ontology and refuses to strand unresolved type references.
Use bulk apply to update dependent definitions in the same write. `--dry-run` previews removal.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Reusable field-type name to remove |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--dry-run` | bool | no | Validate and preview the change without writing (default: `false`) |
| `--check-dependent-bundles` | bool | no | Examine every registered consumer of a shared ontology sidecar (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.

Supported global options: `--bundle-id`, `--json`.

Reusable field types are ontology-wide definitions. Add/update `--from` accepts a whole definition; update replaces it completely. Inspect consumers before removal or tightening. `--dry-run` validates the proposed ontology without publishing the sidecar; coordination state may be created.
