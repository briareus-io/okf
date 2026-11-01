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
For claim attribution, define the Markdown footnote and use its label as `sources[].id`.
The check ignores footnote-like text in code and references without a definition.

## Resolution and recorded observations

The CLI adds optional `kind` and `fingerprint` extensions for change detection. Resource
namespaces are explicit:

| Source kind | Resource interpretation |
| --- | --- |
| `git-path`, `git-commit` | Repository-relative Git material. |
| `file`, `line-range`, `markdown-heading` | Bundle-root-relative files or selected content. |
| `bundle-resource` | An `okf://<bundle-id>/<path>` resolved through the selected catalog and scope. |

Do not interchange these namespaces or copy sources merely to make artifact resolution
succeed. A `bundle-resource` fingerprint records content identity and available immutable
origin information. A changed lock can select a different revision with identical file bytes;
revision identity and content drift are separate observations. Resolving or refreshing a source
never silently changes the authored URI into a machine-specific cache path.

A source may declare `bundle_ref.snapshot` when the claim requires a particular commit or
content digest. Resolving its location, satisfying that snapshot expectation, and verifying
the claim are separate checks. If the selected lock differs from the required snapshot,
available content remains a labeled candidate or mismatch. Refreshing a fingerprint does not
accept a conflicting snapshot expectation or move the dependency lock.

## Live pointers and evidence

A URL-valued field can identify a live destination, such as a current project status page,
without copying its contents into the concept. Use field names or a modeled object to make
that purpose explicit; not every URL is a live pointer. A pointer is ordinary metadata, not
a special CLI concept type or a promise of freshness.

Storing or reading the pointer does not fetch its target. When the user's task and bundle
guidance call for consulting it, distinguish the live destination from the resulting
observation. Record the observation time and available version evidence when using that
information to support a claim. Without consultation, do not assert the destination's current
contents. Ordinary sources can change too; a URL alone does not preserve historical evidence.

## Reconcile source changes

This bundle cites current repository files and keeps their source kinds for resolution and
impact analysis. It does not store fingerprint baselines or links to old Git snapshots.
Without a recorded baseline, a source cannot be classified as drifted; use known changes
with `affected` and review the current source directly.

[stale](../commands/stale.md) reports source drift and lifecycle expiry;
[affected](../commands/affected.md) finds dependents of supplied changes.
[refresh](../commands/refresh.md) records supported current fingerprints after reconciliation.
It neither verifies the prose nor clears expired review dates.
Follow [maintain source-backed knowledge](../guides/maintain-source-backed-knowledge.md).
