---
type: Command
group: mutate
mutates: true
implemented_by:
- /components/mutate
title: okf mv
description: Moves or renames a concept and rewrites every inbound link.
sources:
- resource: crates/okf-cli/src/commands/mutate.rs
  kind: git-path
  fingerprint:
    blob_sha: 73e747e096148161676c75253cff3556a2deaff7
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
  fingerprint:
    blob_sha: a99f651a30e3a6ad2859066a5b8f287e73391371
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
  fingerprint:
    blob_sha: fad8084ae5833fd506b4e8818e62e0a2d387c2d6
last_modified: 2026-10-04T20:39:29Z
---
# okf mv

Moves or renames a concept and rewrites body links, ontology references, and standard internal `sources[].resource` paths. It also rebases the moved concept's relative standard paths. Repository-relative `git-path` and `git-commit` source resources remain unchanged.

Moves rewrite declared nested-reference occurrences and curated navigation within the selected bundle.
Publication is journaled per file. Cross-bundle move orchestration remains unsupported.
Inspect explicit-scope consumers and coordinate reviewed changes across bundles manually; a
single-bundle move cannot establish that every external consumer was repaired.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<old>` | positional | yes | Existing concept id |
| `<new>` | positional | yes | New concept id |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.
