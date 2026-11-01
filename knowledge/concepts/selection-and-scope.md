---
type: Topic
audience: user
title: Selection and scope
description: Select one primary bundle and explicitly choose any additional examination scope.
sources:
- resource: crates/okf-core/src/bundle/context.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/snapshot_context.rs
  kind: git-path
---
# Selection and scope

Selection chooses the primary bundle; scope chooses which additional registered bundles a
supported command may examine. A trailing path and `--bundle-id` are alternative selectors.
`OKF_BUNDLE` always holds a path. See [configuration](../reference/configuration.md) for precedence.

Registering another bundle does not authorize automatic traversal. Request it with
`--scope-bundle` or deliberately use `--catalog-scope` on commands that support scope.
Report requested scope, examined material, and unavailable members separately.

## References select targets, not examination scope

Ordinary cross-bundle paths retain their filesystem-layout meaning. A logical authored link
such as `okf://acme.policy/policies/access.md` instead identifies a registered bundle and a
path inside it. A qualified CLI target such as `acme.policy:/policies/access` is command
syntax, not the authored URI. The catalog resolves identity and location; a Git registration's
lock selects accepted bytes. No reference automatically acquires a dependency or broadens scope.

## Working trees, accepted locks, and historical inspection

| Selection | What it reads |
| --- | --- |
| Ordinary directory registration | Its current working-tree content, which can change between commands. |
| Ordinary managed Git registration | Its available accepted snapshot from the selected catalog lock. |
| `--workspace-revision <ref>` | The consumer repository's historical catalog, lock, and local content; remote commits come from that historical lock. |
| `--bundle-revision <id>=<commit>` | Explicitly inspected local objects for that bundle, with any divergence from the accepted lock reported. |

Use these selectors only on commands that advertise them in help or `okf schema`.
Legacy `--revision` remains available on supported operations, but a single revision string
must not ambiguously select unrelated repositories. Historical reads never fetch missing
objects or accept a new lock. External directory registrations can remain working-tree
context even when a consumer catalog is historical; preserve the reported evidence.

A source's snapshot expectation is separate from query selection. Available current content
does not satisfy a known mismatch, verify claims, or accept source drift. See
[sources and provenance](sources-and-provenance.md) for that distinction.

Use [multiple bundles](../guides/work-with-multiple-bundles.md) to build a local workspace and
[remote dependencies](../guides/remote-dependencies.md) for explicit Git acquisition and updates.
This repository uses a single bundle with separate reader and contributor paths.
