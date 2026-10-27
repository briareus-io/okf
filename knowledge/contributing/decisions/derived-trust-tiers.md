---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/verification-and-freshness
title: Derive trust tiers from verification evidence
description: Expose recorded review actors without treating labels as proof of truth.
---
# Derive trust tiers from verification evidence

## Context

A manually asserted trust label could contradict the document review history.

## Choice

Derive the tier from verified actors: human: gives human-reviewed, other actors machine-confirmed, and no verification unverified. Interpret generation, lifecycle and source drift separately.

## Alternatives

An editable tier is simpler to set but disconnects the label from its evidence.

## Consequences

Recorded actors are not authenticated identities and the label does not prove truth. A generated or changed document needs review of whether old verification remains current.

## Reconsider when

Revisit if the verification evidence model gains authenticated identities; do not imply authentication today.

## Related guidance

- [Verification and freshness](../../concepts/verification-and-freshness.md)
