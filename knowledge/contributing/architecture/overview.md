---
type: Reference
audience: contributor
title: Architecture overview
description: Understand the deterministic core, CLI adapter, effect ports and agent workflows.
---
# Architecture overview

`okf-core` owns deterministic document, query, graph, validation and mutation behavior.
`okf-cli` adapts arguments, configuration, output streams and exit codes. Agent skills provide
judgment: choosing evidence, interpreting changes, authoring explanations and performing reviews.
Filesystem, Git, clock and network effects cross explicit ports so core behavior can be tested
with controlled implementations.

The [module map](modules.md) locates implementation responsibilities. The
[boundary decision](../decisions/architectural-boundaries.md) explains the separation;
[derived schema](../decisions/derived-schema.md) explains why documentation comes from the CLI
contract. Use [development](../development.md) for build and validation commands.

Concept metadata remains open. Local ontology and catalog settings interpret it without
changing OKF conformance. Source resolution, source drift and claim verification remain distinct.
Mutations prepare changes before journaled publication; ordinary readers do not receive a
multi-file atomic snapshot. See [command contracts](../../reference/command-contracts.md).

Use the [accepted decisions](../decisions/index.md) and current implementation when changing
these boundaries. Historical intent and architecture drafts are not part of this manual.
