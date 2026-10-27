---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/selection-and-scope
title: Catalog identity and explicit scope
description: Catalog coordination preserves ordinary paths and separates selection, examination and snapshot evidence.
sources:
- resource: knowledge/contributing/design/multi-bundle-capability.md
  kind: git-path
- resource: crates/okf-core/src/graph/catalog.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/catalog.rs
  kind: git-path
- resource: crates/okf-core/src/bundle/config.rs
  kind: git-path
---
# Catalog identity and explicit scope

## Context

Several local bundles need coordinated inspection without silently broadening every query.

## Choice

A bundle remains an ordinary directory with path-based concept IDs. The optional catalog adds
namespaced identity and local locations; it does not require an identity manifest or an ontology.
Conflicting or overlapping registrations are configuration errors, not content conformance errors.

Primary selection and examination scope are separate. Catalog registration alone does not imply
traversal. Report requested scope, actual examined material and unavailable members. Ordinary
relative links retain layout semantics across registered roots, so distributing one bundle alone
may break links. Never infer equivalence from matching names.

Optional `bundle_ref` source metadata must agree with ordinary `resource` meaning. Snapshot
expectations and ordinary live targets are separate evidence. A current candidate never fulfills
a known mismatch or accepts itself as a substitute. Local Git inspection must not fetch objects;
resolution does not verify claims, refresh fingerprints, or execute computation artifacts.

## Alternatives

Implicit catalog-wide traversal is convenient but hides examination scope. Replacing ordinary paths with catalog IDs would change stored resource semantics.

## Consequences

The explicit limits are part of the contract. Inspect incomplete or unavailable evidence rather than treating it as a successful empty result.

## Reconsider when

Revisit when a concrete workflow requires a broader contract, preserving explicit scope and meaning.

## Related guidance

- [Selection and scope](../../concepts/selection-and-scope.md)
