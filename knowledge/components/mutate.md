---
type: Component
layer: core
depends_on:
- /components/bundle
- /components/check
- /components/fingerprint
- /components/model
- /components/ontology
- /components/graph
- /components/parse
- /components/ports
title: mutate module
description: Atomic, self-validating concept writes that preserve unknown YAML meaning and body text while applying bundle mutations.
sources:
- resource: crates/okf-core/src/mutate/mod.rs
  kind: git-path
  fingerprint:
    blob_sha: a95297903e7bf61c1fcdd9fd6abaf8b6dccae841
- resource: crates/okf-core/src/mutate/structured.rs
  kind: git-path
  fingerprint:
    blob_sha: 6c504c52f0f9f23b83fde8b312aae82dafcc8c78
- resource: crates/okf-core/src/mutate/transaction.rs
  kind: git-path
  fingerprint:
    blob_sha: 07cc111b621d96b7988f0c7e0613b9ccd8984061
- resource: crates/okf-core/src/mutate/gate.rs
  kind: git-path
  fingerprint:
    blob_sha: 94338090c19cce174b9b258703279060a74a9544
- resource: crates/okf-core/src/mutate/refresh.rs
  kind: git-path
  fingerprint:
    blob_sha: 7046e2f8c2e77da8fdd832eb1ee5b1eb4c3098df
last_modified: 2026-10-05T10:16:06Z
---
# mutate module

Atomic, self-validating concept writes preserve unknown YAML meaning and body text while applying init, add, edit, mv, rm, verify, and refresh. They compose [bundle](bundle.md), [check](check.md), [fingerprint](fingerprint.md), [model](model.md), [ontology](ontology.md), [graph](graph.md), [parse](parse.md), and effect [port](ports.md) services. YAML comments and scalar presentation may normalize.

Location: `crates/okf-core/src/mutate`.

Structured frontmatter mutations operate on YAML values before persistence. Object paths create
missing maps and reject list traversal; RFC 6902 patches use concrete JSON Pointer addresses.
All patch operations and concept conformance checks complete before writing. Dry-run applies the
same preflight without publishing authored content or lifecycle changes; coordination locks
and temporary staging may be created. Meaningful writes preserve
existing verification invalidation and generated timestamp semantics.

In 0.5.0, CLI direct and changeset writers prepare a final state and publish
through the journaled single-bundle protocol. An enabled ontology gate
compares new or worsened findings independently from advisory lint display.
Baseline acquisition can produce a guarded token without an operation plan;
it may create coordination state. Single refresh computes all required
fingerprints before publication and leaves the concept unchanged if a
required source fails. Recovery restores the recorded transaction state
without treating an interrupted output stream as proof of rollback.
The 0.5 write gate blocks with `write-gate-incomplete` when an out-of-bundle
typed reference target cannot be checked. Explicit dependent-bundle
examination for a shared ontology does not imply ordinary cross-bundle
reference traversal.
