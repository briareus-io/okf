---
type: Topic
audience: user
title: Selection and scope
description: Select one primary bundle and explicitly choose any additional examination scope.
sources:
- resource: crates/okf-core/src/bundle/resolve.rs
  kind: git-path
---
# Selection and scope

Selection chooses the primary bundle; scope chooses which additional registered bundles a
supported command may examine. A trailing path and `--bundle-id` are alternative selectors.
`OKF_BUNDLE` always holds a path. See [configuration](../reference/configuration.md) for precedence.

Registering another bundle does not authorize automatic traversal. Request it with
`--scope-bundle` or deliberately use `--catalog-scope` on commands that support scope.
Report requested scope, examined material, and unavailable members separately.

Ordinary cross-bundle paths retain their layout meaning. A qualified CLI target such as
`acme.policy:/policies/access` is a selector, not an authored Markdown link. Snapshot
expectations are distinct from current candidate content: a live candidate does not satisfy
a known mismatch, verify claims, or accept source drift.

Use [multiple bundles](../guides/work-with-multiple-bundles.md) when ownership or distribution
requires them. This repository uses a single bundle with separate reader and contributor paths.
