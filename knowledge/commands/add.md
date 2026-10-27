---
type: Command
group: mutate
mutates: true
title: okf add
description: Adds a new concept document, scaffolded from the ontology so required fields and reference keys start present.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/structured.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/structured.rs
  kind: git-path
last_modified: 2026-10-05T10:16:06Z
---
# okf add

Adds a new concept document, scaffolded from the ontology so required fields and reference keys start present. `--attested` creates exact `type: Attested Computation` and requires a runtime plus either `--computation` or `--inline-computation`.

Structured creation supports `--set-yaml key=value` for complete YAML values and `--set-path
path=value` for object-only nested properties. Inputs may be inline, `@file`, or `-`; JSON syntax
works. Object paths create missing maps and reject scalar/list traversal. Scalar `--set` remains
literal. `--dry-run` validates and previews without creating the document.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<path>` | positional | yes | Bundle-relative path of the new concept (with or without `.md`) |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--type <value>` | string | no | Concept `type` (an ontology concept-type key). Optional with `--attested` |
| `--title <value>` | string | no | Concept title |
| `--description <value>` | string | no | Concept description |
| `--body <value>` | string | no | Replace the generated Markdown body. Use `@file` or `-` for stdin |
| `--attested` | bool | no | Scaffold exact `type: Attested Computation`; requires `--runtime` (default: `false`) |
| `--set <value>` | list<string> | no | Set a custom scalar field at creation, `key=value` (repeatable) |
| `--set-yaml <value>` | list<string> | no | Replace a field with YAML: `key=value`, `key=@file`, or `key=-` (repeatable) |
| `--set-path <value>` | list<string> | no | Set an object path with a YAML value, e.g. `deadline.within=72` (repeatable) |
| `--dry-run` | bool | no | Validate and show the resulting diff without writing (default: `false`) |
| `--ref <value>` | list<string> | no | Set a declared reference at creation, `key=link` (repeatable) |
| `--add-source <value>` | list<string> | no | Add a standard source, `resource=<path-or-uri>[,kind=<extension>][,id=...,...]` |
| `--add-source-json <value>` | list<string> | no | Add a full source mapping as JSON/YAML or `@file` (repeatable) |
| `--runtime <value>` | string | no | Runtime for an exact `type: Attested Computation` |
| `--parameter <value>` | list<string> | no | Declared parameter `name:type[:required]` (repeatable) |
| `--computation <value>` | string | no | Path to a computation file; omit to scaffold one inline computation fence |
| `--inline-computation <value>` | string | no | Inline sanctioned computation text, literal, `@file`, or `-` for stdin |
| `--executor-resource <value>` | string | no | Executor instructions/code resource |
| `--receipt <value>` | list<string> | no | Required executor receipt field (repeatable) |
| `--attester-resource <value>` | string | no | Deterministic attester code resource |
| `--generated-by <value>` | string | no | Actor that generated this content |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.

Supported global options: `--bundle-id`, `--json`.

Creation scaffolds a concept; `--body @file` supplies Markdown in the same write (literal text and `-` for stdin also work). `--set-yaml key=value` sets structured YAML/JSON values, with `@file` or `-` input; `--set-path path=value` sets an object property and creates missing maps. Existing `--set` remains literal/scalar. `--dry-run` previews without writes. Input failures leave no skeleton concept. `--generated-by` records the supplied actor and the CLI's current timestamp. Do not invent a historical generation time or actor. For a requested computation only, `--attested --runtime <runtime>` selects exact `Attested Computation`; declare actual parameters with repeatable `--parameter name:type:required` (omit `:required` when optional). Choose `--computation <resource>` or `--inline-computation @file`, then add reviewed executor/receipt/attester fields as needed. These describe a contract and authorize no execution.
