---
type: Topic
audience: user
title: Verification and freshness
description: Distinguish document review, lifecycle, source drift and computation execution.
sources:
- resource: crates/okf-core/src/model/trust.rs
  kind: git-path
---
# Verification and freshness

Trust tiers are derived from recorded verification actors: `human:` actors yield human-reviewed,
other actors yield machine-confirmed, and no verification yields unverified. These are labels
for recorded evidence, not authentication of the actor or guarantees of truth.

Inspect lifecycle separately: absent `status` means stable; a concept can be draft or deprecated,
and `stale_after` can expire. Compare generation and verification times before treating a past
review as current. A source fingerprint can drift independently of all those fields.

Use [verify](../commands/verify.md) only to record a review actually performed by the named actor.
Validation checks document conformance; lint checks conventions; refresh updates source baselines.
None substitutes for that review. A meaningful edit can invalidate earlier verification.

An Attested Computation definition is also a document. [computation check](../commands/computation-check.md)
inspects its contract and reports execution as not run; it is not a runtime attestation.
See [checking results](../reference/troubleshooting.md) and the
[trust decision](../contributing/decisions/derived-trust-tiers.md).
