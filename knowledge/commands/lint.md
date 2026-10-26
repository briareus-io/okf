---
type: Command
group: check
mutates: false
implemented_by:
- /components/check
title: okf lint
description: 'Advisory checks where the opinions live: broken links, missing descriptions, orphans, and ontology violations, at error/warn/info severities with a --fail-on threshold.'
sources:
- resource: crates/okf-cli/src/commands/check.rs
  kind: git-path
  fingerprint:
    blob_sha: 3a0c6b86996b70113dba53ddf52c81b7aac433cf
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
  fingerprint:
    blob_sha: a99f651a30e3a6ad2859066a5b8f287e73391371
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
  fingerprint:
    blob_sha: fad8084ae5833fd506b4e8818e62e0a2d387c2d6
- resource: crates/okf-core/src/mutate/gate.rs
  kind: git-path
  fingerprint:
    blob_sha: 94338090c19cce174b9b258703279060a74a9544
last_modified: 2026-10-04T20:39:29Z
---
# okf lint

Advisory checks where the opinions live: broken links, missing descriptions, orphans, and ontology violations, at error/warn/info severities with a --fail-on threshold.

Recursive ontology diagnostics retain the `ontology-violation` rule with concrete field paths
and detailed metadata codes. Invalid schema configuration is separate from advisory findings;
unsupported constraints and depth/finding-budget exhaustion remain visible. Local index-coverage
checks diagnose omitted immediate children and concept-containing directories without rewriting
curated navigation. Catalog-aware checks retain explicit examination scope.

Read-only lint can examine explicit catalog scope. `lint --fix` in CLI 0.5
requires one selected writable bundle and rejects catalog/scope flags. Each
bundle's fixes are a separate transaction; a later failure cannot roll back an
earlier invocation. Display severity does not weaken an independently enabled
ontology write gate.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fix` | bool | no | Normalize frontmatter order bundle-wide for the enabled frontmatter-order rule (default: `false`) |
| `--fail-on <value>` | string | no | Severity threshold that makes the run fail (exit 1): never|info|warn|error|any (default: `error`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `finding,scope,fix`.
