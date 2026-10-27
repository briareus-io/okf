---
type: DesignDecision
decision_status: accepted
audience: contributor
explains:
- /concepts/sources-and-provenance
title: Use Git through its CLI
description: Keep Git an optional runtime dependency behind an effect port.
---
# Use Git through its CLI

## Context

Git-backed provenance and revision reads need repository behavior without making every local operation depend on a Git library.

## Choice

Invoke Git through the Git port. Avoid gix/git2/libgit2 build dependencies. Git-based fingerprinting and revision operations require Git on PATH; other local operations remain usable without it.

## Alternatives

Embedding a Git library could reduce subprocess overhead but adds a separate dependency and compatibility surface.

## Consequences

Git failures need explicit environment diagnostics. Reuse subprocess work within an invocation where possible, and do not fetch missing objects during local inspection.

## Reconsider when

Revisit if measured subprocess costs or deployment requirements outweigh the dependency benefit.

## Related guidance

- [Sources and provenance](../../concepts/sources-and-provenance.md)
