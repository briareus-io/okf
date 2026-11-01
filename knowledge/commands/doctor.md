---
type: Command
group: check
mutates: true
title: okf doctor
description: Diagnoses conformance and compatibility changes before upgrading an existing bundle, with conservative safe repairs.
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf doctor

Inventories an existing bundle with a tolerant raw-byte walker, reports hard conformance blockers and behavior changes, and can apply only allow-listed safe repairs when explicitly confirmed.

## Configuration migration

Configuration migration is explicit and separate from `--fix-safe`. Use
`okf doctor --migrate-config <okf.toml> --migration-plan <plan.yaml>` to inspect moves,
path rebases, conflicts, and retained legacy values. Supply `--default-bundle-id <id>`
when the old path-only default needs a registration identity. Apply the reviewed plan
with `okf doctor --migration-apply <plan.yaml>`; stale input digests prevent applying
an obsolete plan. See the [0.6 migration guide](../reference/migrations/0.6.0.md).

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--default-bundle-id <value>` | string | no | — |
| `--migrate-config <value>` | string | no | Plan legacy TOML migration from this exact configuration file |
| `--migration-plan <value>` | string | no | Save a migration plan to this path |
| `--migration-apply <value>` | string | no | Explicitly apply a saved configuration migration plan |
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--target <value>` | string | no | Target OKF version (default: `0.2`) |
| `--fix-safe` | bool | no | Enable the allow-listed safe repair set (default: `false`) |
| `--normalize-relative-links` | bool | no | Explicitly normalize bare body links to their current document-relative targets; preview unless --yes (default: `false`) |
| `--dry-run` | bool | no | Show requested repairs without writing (the default without --yes) (default: `false`) |
| `--yes` | bool | no | Confirm applying --fix-safe or --normalize-relative-links changes non-interactively (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `doctor-finding,doctor-summary,doctor-normalization,migration-plan,migration-applied`.

Supported global options: `--bundle-id`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--scope-bundle`.

Inspect migration findings without changing files by default. Catalog-aware source checks require explicit `--scope-bundle` or `--catalog-scope` for other registered bundles. File, line-range, and markdown-heading source paths use the bundle root, consistently with fingerprinting and graph resolution; kind-less provenance and body links use document-relative paths. A leading slash in a typed file source retains filesystem-absolute semantics, unlike a leading slash in a Markdown link. Correct typed sources that previously relied on graph resolution treating them as document-relative.

`--normalize-relative-links` explicitly chooses current document-relative semantics for bare body links and previews exact mappings and diffs; add `--yes` to publish, while `--dry-run` always previews. It does not infer original author intent and is separate from `--fix-safe`. Only checked existing local targets are normalized; missing, blocked, and unrecognized or ambiguous destination syntax remain for review. Destination-only rewrites preserve frontmatter, source entries, labels, titles, and query/fragment suffixes. Normalization is published before any requested safe repairs, in a separate transaction; the two are not one atomic operation. Review completion evidence if later checks or repairs fail.
