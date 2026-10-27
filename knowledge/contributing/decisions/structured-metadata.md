---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/ontology-and-relationships
title: Declared structured metadata interpretation
description: Nested constraints and semantic references remain optional local interpretation rather than OKF conformance.
sources:
- resource: knowledge/contributing/design/structured-metadata-capability.md
  kind: git-path
- resource: crates/okf-core/src/query/selector.rs
  kind: git-path
- resource: crates/okf-core/src/graph/relationships.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/schema.rs
  kind: git-path
---
# Declared structured metadata interpretation

## Context

Useful custom metadata needs queryable structure without imposing a universal domain model.

## Choice

One selector grammar walks `serde_yaml::Value` directly: dotted property names, explicit `[]`
list traversal, and JSON bracket-quoted literal keys. It does not infer traversal, evaluate code,
or expose a general JSONPath language. Occurrence paths preserve same-record pairing.

Named field types compose nested lists and objects. Explicit fields, values and item definitions
replace inherited counterparts. Recursive lint checks exact YAML types and declared constraints;
root metadata remains open and only explicitly closed custom objects reject unknown keys.
Configuration errors, unsupported constraints and incomplete checks remain distinguishable.

Declared selectors alone make custom metadata references. Semantic relationships carry authored
conditions; inverse backlinks do not express consent or compliance. Typed query filters combine
with AND, facets concern examined matches before paging, and bounded expansion reports gaps.
Computed display and projection records preserve authored metadata. Index coverage is a read-only
local navigation check; optional indexes do not become a conformance requirement.

[Explicit structured mutation inputs](structured-authoring.md) now author these values and
declarations through CLI YAML inputs, object paths, concrete patches and atomic ontology changes.
Query selectors and inverse views retain their read interpretation; mutation does not assert
verification, target consent, or policy compliance.

## Alternatives

Inferring references from path-looking strings is ambiguous. A general executable query language adds complexity beyond the bounded selector model.

## Consequences

The explicit limits are part of the contract. Inspect incomplete or unavailable evidence rather than treating it as a successful empty result.

## Reconsider when

Revisit when a concrete workflow requires a broader contract, preserving explicit scope and meaning.

## Related guidance

- [Ontology and relationships](../../concepts/ontology-and-relationships.md)
