---
type: Topic
audience: user
title: Sources and provenance
description: Separate source attribution, source resolution and recorded change baselines.
sources:
- resource: crates/okf-core/src/fingerprint/mod.rs
  kind: git-path
---
# Sources and provenance

A standard source entry needs `resource`. Optional IDs connect claim footnotes to specific
sources. Attribution records where information came from; it does not prove that the claim
is correct or that the source is still available.

The CLI adds optional `kind` and `fingerprint` extensions for change detection. Repository
`git-path` and `git-commit` resources resolve from the Git worktree root. `file`, `line-range`,
and `markdown-heading` resources resolve from the bundle root. Do not interchange those
namespaces or copy source files merely to make artifact resolution succeed.

This bundle cites current repository files and keeps their source kinds for resolution and
impact analysis. It does not store fingerprint baselines or links to old Git snapshots.
Without a recorded baseline, a source cannot be classified as drifted; use known changes
with `affected` and review the current source directly.

[stale](../commands/stale.md) reports source drift and lifecycle expiry;
[affected](../commands/affected.md) finds dependents of supplied changes.
[refresh](../commands/refresh.md) records supported current fingerprints after reconciliation.
It neither verifies the prose nor clears expired review dates.
Follow [maintain source-backed knowledge](../guides/maintain-source-backed-knowledge.md).
