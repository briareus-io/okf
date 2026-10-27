---
type: Command
group: query
mutates: false
title: okf ontology field-type show
description: Inspect one authored reusable field type and its effective definition.
sources:
- resource: crates/okf-cli/src/commands/ontology.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
---
# okf ontology field-type show

Inspect one authored reusable field type and its effective definition.

See the [complete CLI reference](index.md) for operation schemas and guarantees.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Concept type name |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `ontology_field_type`.

Supported global options: `--bundle-id`, `--json`.

Inspect authored reusable definitions and resolved inherited constraints. JSON includes both definition and effective fields.
