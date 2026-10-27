---
type: Command
group: mutate
mutates: true
title: okf ontology add
description: Defines a new concept type with its typed fields and reference rules, written to ontology.yaml through the lossless ontology editor.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/edit.rs
  kind: git-path
- resource: crates/okf-cli/src/structured.rs
  kind: git-path
last_modified: 2026-10-05T10:16:06Z
---
# okf ontology add

Defines a new concept type with its typed fields and reference rules, written to ontology.yaml through the lossless ontology editor.

Structured declaration flags `--field-yaml`, `--ref-yaml`, and `--relationship-yaml` accept
`name=value` with inline YAML/JSON, `@file`, or stdin `-`. Each supplied declaration replaces its
whole named entry. `--from` accepts one concept-type definition; omitted named entries remain
on update. Overlapping file/flag operations fail. `--remove-relationship` is available on update.
The combined prospective ontology validates before publication; `--dry-run`
previews without publishing the sidecar, though coordination state may be created.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<name>` | positional | yes | Concept type name |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--description <value>` | string | no | Description of the concept type |
| `--field <value>` | list<string> | no | A typed field, `key:type[:required][:v1|v2|...]` (repeatable) |
| `--ref <value>` | list<string> | no | A reference rule, `key:Target[|Target2]:cardinality` (repeatable) |
| `--remove-field <value>` | list<string> | no | Remove a typed field (update only; repeatable) |
| `--remove-ref <value>` | list<string> | no | Remove a reference rule (update only; repeatable) |
| `--field-yaml <value>` | list<string> | no | A complete field declaration, key=YAML|@file|- (repeatable) |
| `--ref-yaml <value>` | list<string> | no | A complete reference declaration, key=YAML|@file|- (repeatable) |
| `--relationship-yaml <value>` | list<string> | no | A complete relationship declaration, key=YAML|@file|- (repeatable) |
| `--remove-relationship <value>` | list<string> | no | Remove a relationship (update only; repeatable) |
| `--from <value>` | string | no | Merge a concept-type definition from inline YAML, @file, a file path, or stdin (-) |
| `--dry-run` | bool | no | Validate and preview the change without writing (default: `false`) |
| `--attested` | bool | no | Mark the exact `Attested Computation` type as standard attested (default: `false`) |
| `--check-dependent-bundles` | bool | no | Examine every registered consumer of a shared ontology sidecar (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.

Supported global options: `--bundle-id`, `--json`.

Built-in field types: `string`, `text`, `int`, `bool`, `date`, `datetime`, `uri`, `enum`, `list`, `object`; custom type names must resolve in the existing sidecar's `field_types`. Reference cardinalities are `0..1` (optional one), `1..1` (exactly one), `0..n` (optional many), and `1..n` (at least one). For example, `--field "stage:enum:draft|active"` declares choices and `--ref "depends_on:Service:0..n"` permits zero or more Service links. Observed presence alone does not justify a required rule. Structured `--field-yaml`, `--ref-yaml`, and `--relationship-yaml` inputs replace complete named declarations. `--from` merges supplied declarations, retaining omitted ones; overlaps with flags fail. `--remove-relationship` deletes a named rule. YAML inputs accept inline values, `@file`, or `-`; JSON syntax works. Preview with `--dry-run`. These are advisory local rules, not conformance requirements.
