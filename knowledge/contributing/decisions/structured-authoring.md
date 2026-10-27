---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/ontology-and-relationships
title: Explicit structured mutation inputs
description: YAML assignments, object paths and concrete patches extend authoring while preserving literal scalar flags.
sources:
- resource: crates/okf-cli/src/structured.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/structured.rs
  kind: git-path
- resource: crates/okf-core/src/ontology/edit.rs
  kind: git-path
- resource: plugins/okf/skills/ontology/references/structured-authoring.md
  kind: git-path
---
# Explicit structured mutation inputs

## Context

Nested metadata needs deliberate updates while existing scalar commands retain their meaning.

## Choice

Existing scalar flags retain literal keys. YAML values (including JSON syntax) make objects,
lists and explicit scalar types authorable without another shorthand grammar. Object-path writes
handle common nested map changes and create missing maps; they reject lists and scalar traversal.
Concrete JSON Pointer patches supply list addressing and guarded RFC 6902 changes. Query selectors
remain separate because wildcard selection does not identify a single mutation address.

Named ontology declarations replace completely; omitted declarations remain on concept update.
Reusable field types replace whole definitions. Dependent bulk changes and explicit removals
validate the final ontology before writing once, avoiding order-dependent intermediate schemas.

Strict input parsing rejects ambiguous duplicate/non-string keys, multiple documents, tags,
aliases and merge keys. Only one input may consume stdin. Dry-run uses the same preflight without
writes. Presentation can normalize; preserved meaning is distinct from exact byte preservation.
Concept lifecycle consequences remain those of meaningful edits; schema validity is advisory and
neither establishes verification nor proves relationship acceptance.

## Alternatives

Overloading scalar flags or using wildcard queries as write addresses would make mutation targets ambiguous.

## Consequences

The explicit limits are part of the contract. Inspect incomplete or unavailable evidence rather than treating it as a successful empty result.

## Reconsider when

Revisit when a concrete workflow requires a broader contract, preserving explicit scope and meaning.

## Related guidance

- [Ontology and relationships](../../concepts/ontology-and-relationships.md)
