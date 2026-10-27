---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/bundles-and-concepts
title: Derive CLI documentation from the command contract
description: Keep discovery and generated reference tied to the implementation.
---
# Derive CLI documentation from the command contract

## Context

Independent command lists and argument tables drift as flags and output contracts evolve.

## Choice

Derive the command surface from the clap definition through okf schema. Generate command Arguments sections, shared contract reference and focused skill references from that output. Keep curated behavior notes alongside the generator.

## Alternatives

Hand-maintained tables offer editorial freedom but create duplicate contract maintenance. Generating all prose would lose task context and judgment.

## Consequences

Argument facts have one source; human task guidance remains authored. Generation checks must fail on missing command pages as well as stale output.

## Reconsider when

Revisit the rendering when adding a new delivery format, while preserving a single command contract.

## Related guidance

- [Bundles and concepts](../../concepts/bundles-and-concepts.md)
