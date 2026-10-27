---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/ontology-and-relationships
title: Preserve authored data through edits
description: Retain unknown metadata and distinguish semantic preservation from YAML presentation.
---
# Preserve authored data through edits

## Context

Concepts carry producer-specific extensions, and ontology files are maintained by people. Rewriting them must not silently discard unrelated meaning.

## Choice

Preserve unknown concept keys and values with ordered mappings. YAML comments, quoting and scalar presentation in frontmatter may normalize. Ontology editing uses typed validation and restores comments by surviving key paths; removed-key comments disappear.

## Alternatives

A fixed metadata struct would discard extensions. Exact byte preservation would require a more complex editing representation; ordinary serialization alone would discard ontology comments.

## Consequences

Semantic preservation is not byte preservation. Authoring order may normalize to ontology order; no guarantee of original whitespace or quoting follows from preserving values.

## Reconsider when

Revisit if users need presentation guarantees that cannot be supplied by the current editor.

## Related guidance

- [Ontology and relationships](../../concepts/ontology-and-relationships.md)
