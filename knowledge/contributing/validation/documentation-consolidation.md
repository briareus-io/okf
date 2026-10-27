---
type: Reference
audience: contributor
title: Documentation consolidation
description: Source-to-destination mapping for the unified manual and rationalized decisions.
---
# Documentation consolidation

The documentation now lives in one bundle, with reader and contributor entry points.
This mapping records the current destinations of intentional moves and merges, and identifies removals.
No review event or source baseline was fabricated during migration.

## Document moves

| Previous path | Destination |
| --- | --- |
| `docs/consistent-command-contracts.md` | [reference/command-contracts](../../reference/command-contracts.md) |
| `docs/command-contracts-validation.md` | [reference/troubleshooting](../../reference/troubleshooting.md) |
| `docs/command-performance.md` | [guides/bounded-reads](../../guides/bounded-reads.md) |
| `docs/multi-bundle-capability.md` | [contributing/design/multi-bundle-capability](../design/multi-bundle-capability.md) |
| `docs/structured-metadata-capability.md` | [contributing/design/structured-metadata-capability](../design/structured-metadata-capability.md) |
| `docs/capability-validation.md` | [contributing/validation/capability-validation](capability-validation.md) |
| `docs/releases.md` | [contributing/release-process](../release-process.md) |
| `docs/0.5.0-migration.md` | [reference/migrations/0.5.0](../../reference/migrations/0.5.0.md) |
| `ARCHITECTURE.md` | Removed historical sketch; current guidance is in [architecture](../architecture/overview.md) |
| `INTENT.md` | Removed historical draft; current choices are in [decisions](../decisions/index.md) |
| `docs/okf-cli-reference.md` | [Command pages](../../commands/index.md) and [generated shared contracts](../../reference/cli-contracts.md) |

## Concept consolidation

| Previous concepts | Destination |
| --- | --- |
| `components/bundle`, `components/check`, `components/fingerprint`, `components/graph`, `components/model`, `components/mutate`, `components/okf-cli`, `components/okf-core`, `components/ontology`, `components/output`, `components/parse`, `components/ports`, `components/query`, `components/render` | [contributing/architecture/modules](../architecture/modules.md) |
| `decisions/check-returns-findings`, `decisions/hands-vs-brain`, `decisions/ports-boundary` | [contributing/decisions/architectural-boundaries](../decisions/architectural-boundaries.md) |
| `decisions/derived-schema` | [contributing/decisions/derived-schema](../decisions/derived-schema.md) |
| `decisions/derived-trust-tiers` | [contributing/decisions/derived-trust-tiers](../decisions/derived-trust-tiers.md) |
| `decisions/git-cli-not-library` | [contributing/decisions/git-cli-not-library](../decisions/git-cli-not-library.md) |
| `decisions/multi-bundle-scope` | [contributing/decisions/multi-bundle-scope](../decisions/multi-bundle-scope.md) |
| `decisions/lossless-frontmatter`, `decisions/lossless-ontology-yaml` | [contributing/decisions/preserve-authored-data](../decisions/preserve-authored-data.md) |
| `decisions/structured-authoring` | [contributing/decisions/structured-authoring](../decisions/structured-authoring.md) |
| `decisions/structured-metadata` | [contributing/decisions/structured-metadata](../decisions/structured-metadata.md) |
| `decisions/validate-vs-lint` | [contributing/decisions/validate-vs-lint](../decisions/validate-vs-lint.md) |

The 25 `types/` concepts were removed; Rust API documentation owns implementation types.
References to them became inline type names where their context remains useful.
The README now links into installation, guides and reference rather than duplicating them.
Command IDs and skill IDs remain stable. Old decision IDs are recorded above rather than retained as duplicate stubs.

## Validation

Run `cargo xtask docs --check`, `cargo xtask skills`, and `python3 scripts/check-docs.py`.
The latter checks navigation and executes worked artifacts in disposable bundles.
Recorded source fingerprints and Git snapshot links were removed. Current repository source
references remain; only the missing-baseline lint rule is disabled. No verification event was added.
