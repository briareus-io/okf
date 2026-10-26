---
type: Command
group: check
mutates: false
implemented_by:
- /components/check
title: okf stale
description: Detects extension fingerprint drift and lifecycle expiry using offset-aware, inclusive stale_after comparison.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
  fingerprint:
    blob_sha: a99f651a30e3a6ad2859066a5b8f287e73391371
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
  fingerprint:
    blob_sha: fad8084ae5833fd506b4e8818e62e0a2d387c2d6
- resource: crates/okf-cli/src/commands/check.rs
  kind: git-path
  fingerprint:
    blob_sha: 3a0c6b86996b70113dba53ddf52c81b7aac433cf
- resource: crates/okf-core/src/check/stale.rs
  kind: git-path
  fingerprint:
    blob_sha: b8f20b40cee8ce839c7acd73c40e32b698844a41
last_modified: 2026-10-04T20:39:29Z
---
# okf stale

Compares recorded source fingerprint extensions against recomputed values and applies exact `now >= stale_after` lifecycle semantics with explicit-offset instant parsing.

CLI 0.5 distinguishes a changed baseline, absent baseline, established missing
resource, unavailable computation, unsupported fingerprint kind, and expiry.
Kindless provenance makes no fingerprint-health claim. `--only` filters
display, `--summary` counts examined and displayed states, and
`--fail-on-state` sets an independent failure predicate. These observations
do not change OKF document conformance.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |
| `--fail-on <value>` | string | no | Existing discovery failure threshold; state policy is separate (default: `never`) |
| `--only <value>` | list<string> | no | Display only these source-health states (comma-separated or repeatable) |
| `--summary` | bool | no | Emit a count of examined, displayed, and omitted findings (default: `false`) |
| `--fail-on-state <value>` | list<string> | no | Fail when any examined source has one of these states |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `drift`.
