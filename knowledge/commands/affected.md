---
type: Command
group: check
mutates: false
title: okf affected
description: 'Impact query: given a set of changed links, computes the blast radius of concepts that may need review via a reverse walk, direct by default or --transitive.'
sources:
- resource: crates/okf-cli/src/commands/check.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf affected

Impact query: given a set of changed links, computes the blast radius of concepts that may need review via a reverse walk, direct by default or --transitive.

Catalog-aware affected analysis walks qualified reverse edges within explicitly examined scope.
Requested and available bundle/version scope accompany results; consumers outside that scope
remain unknown. Snapshot mismatch, ordinary target resolution and fingerprint change are
separate results. A current candidate is not an accepted substitute.

CLI 0.5 reads piped changed resources only when `--changed -` is explicit, or
when no changed arguments were supplied and the stdin-only fallback applies.
`--changed-base` selects the bundle (default), repository worktree or filesystem
namespace for every changed input. `--depth` requires `--transitive`.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--changed <value>` | list<string> | no | A changed link/concept-id/resource (repeatable; use - to read stdin) |
| `--changed-base <value>` | string | no | Interpret changed paths relative to the bundle, repository, or invocation directory (default: `bundle`) |
| `--transitive` | bool | no | Follow the cascade past direct dependents (default: `false`) |
| `--depth <value>` | int | no | Cap hops with --transitive |
| `--fail-on <value>` | string | no | Fail (exit 1) on any affected concept: never (default) | info | warn | error | any (default: `never`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `affected,scope,bundle-affected`.

Supported global options: `--bundle-id`, `--catalog-scope`, `--json`, `--revision`, `--scope-bundle`.
