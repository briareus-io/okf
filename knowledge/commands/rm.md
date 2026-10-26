---
type: Command
group: mutate
mutates: true
implemented_by:
- /components/mutate
title: okf rm
description: Removes a concept, refusing if backlinks would dangle unless --force is given.
sources:
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
# okf rm

Removes a concept, refusing if backlinks would dangle unless --force is given.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id to remove |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--force` | bool | no | Remove even if backlinks would dangle (default: `false`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.
