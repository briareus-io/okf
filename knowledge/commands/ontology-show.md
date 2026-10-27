---
type: Command
group: query
mutates: false
title: okf ontology show
description: Shows one concept type's fields and typed reference rules with cardinality.
sources:
- resource: crates/okf-cli/src/commands/ontology.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf ontology show

Shows one concept type's fields and typed reference rules with cardinality.

Effective definition inspection resolves reusable field types recursively, including inherited
constraints and nested object/list definitions. Explicit fields, values or item definitions
replace inherited counterparts. Each selected bundle uses its own effective ontology context.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Concept type name |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `ontology_type`.

Supported global options: `--bundle-id`, `--json`.
