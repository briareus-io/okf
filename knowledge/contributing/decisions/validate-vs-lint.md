---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/ontology-and-relationships
- /concepts/verification-and-freshness
title: Separate conformance from local quality policy
description: Keep valid OKF portable across different local modeling choices.
---
# Separate conformance from local quality policy

## Context

Users need to distinguish an invalid document format from a valid document that violates a local convention.

## Choice

Keep validate limited to format conformance. Use lint for advisory links, descriptions, ontology and other local rules; configure severity separately. Optional mutation gates enforce local policy without redefining the format.

## Alternatives

A single strict validator would make local conventions look like universal OKF requirements.

## Consequences

Conformance alone says nothing about source truth, freshness or local quality. Callers choose checks and failure thresholds explicitly.

## Reconsider when

Revisit only when the underlying format changes, not when a repository adds another convention.

## Related guidance

- [Ontology and relationships](../../concepts/ontology-and-relationships.md)
- [Verification and freshness](../../concepts/verification-and-freshness.md)
