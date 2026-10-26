---
type: Command
group: mutate
mutates: true
implemented_by:
- /components/mutate
title: okf refresh
description: Re-records source fingerprints after a change has been acknowledged, clearing the drift that stale would report.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
  fingerprint:
    blob_sha: a99f651a30e3a6ad2859066a5b8f287e73391371
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
  fingerprint:
    blob_sha: fad8084ae5833fd506b4e8818e62e0a2d387c2d6
- resource: crates/okf-cli/src/commands/mutate.rs
  kind: git-path
  fingerprint:
    blob_sha: 73e747e096148161676c75253cff3556a2deaff7
- resource: crates/okf-core/src/mutate/refresh.rs
  kind: git-path
  fingerprint:
    blob_sha: 7046e2f8c2e77da8fdd832eb1ee5b1eb4c3098df
last_modified: 2026-10-04T20:39:29Z
---
# okf refresh

Re-records extension source fingerprints after a change has been acknowledged. It does not overwrite standard source modification time or clear an expired `stale_after`.

CLI 0.5 prepares every required fingerprint before publication. Missing,
unavailable or unsupported required sources block the whole single-concept
update; successfully computed fingerprints are not published in that case.
Kindless sources are provenance only and do not request fingerprinting. A
refresh remains separate from document verification.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<concept>` | positional | yes | Concept id to refresh |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fail-on <value>` | string | no | Fail (exit 1) when any source is skipped: never (default) | skipped | any (default: `never`) |
| `--require-ontology-gate` | bool | no | Require the ontology write gate even when bundle policy leaves it disabled (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `change`.
