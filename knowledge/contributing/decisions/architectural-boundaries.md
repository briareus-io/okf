---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/bundles-and-concepts
title: Separate core, CLI and judgment
description: Keep deterministic logic reusable and effects testable.
---
# Separate core, CLI and judgment

## Context

The CLI, library and agent workflows have different responsibilities. Coupling them would make reuse and testing depend on terminal or agent behavior.

## Choice

Keep deterministic operations in core, arguments/output/exit codes in the CLI, and judgment in skills. Route filesystem, Git, clock and network effects through ports. Core returns findings with severities; the CLI maps results and thresholds to process exit codes.

## Alternatives

Putting everything in CLI handlers is simpler initially but prevents library reuse. Letting core print or exit couples consumers to one interface. Direct effects make tests depend on the environment.

## Consequences

Ports and adapters require some plumbing, but enable controlled tests and other consumers. A successful check still needs interpretation under its failure threshold.

## Reconsider when

Revisit if a new consumer needs a different operation boundary; keep effect and judgment responsibilities explicit.

## Related guidance

- [Bundles and concepts](../../concepts/bundles-and-concepts.md)
