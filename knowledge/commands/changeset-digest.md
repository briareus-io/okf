---
type: Command
group: query
mutates: false
title: okf changeset digest
description: Reads a guarded baseline token for a selected bundle without an operation document.
sources:
- resource: crates/okf-core/src/mutate/changeset.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/changeset.rs
  kind: git-path
---
# okf changeset digest

Returns the opaque baseline token used by guarded changeset application. It does not publish
authored content. Acquiring a guarded baseline may create coordination state and requires the
corresponding filesystem permissions. The token covers the inputs declared by the changeset baseline
contract; a later apply must recapture and compare the covered state. It is not a snapshot for
arbitrary follow-up reads.

With `--scope-bundle` or `--catalog-scope`, the token also covers examined target bundles
and the catalog/configuration inputs that identify and interpret them. Use the same
scope on the later apply. An unrelated file change within an examined bundle can
invalidate the digest; obtaining a digest does not validate a proposed concept's types.
See the [scoped write-gate contract](../reference/command-contracts.md#apply-a-local-ontology-write-policy)
for coverage, conflict handling, and consistency limits.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `baseline`.

Supported global options: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

Returns the baseline token without an operation document. Use the same `--scope-bundle` or `--catalog-scope` as the later scoped plan/apply to cover its examined target inputs. The token does not establish ontology correctness or approve an operation document; apply validates the proposed changes and rechecks its covered baseline.
