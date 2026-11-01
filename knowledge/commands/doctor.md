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
| `--dry-run` | bool | no | Show safe repairs without writing (the default without --yes) (default: `false`) |
| `--yes` | bool | no | Confirm applying --fix-safe changes non-interactively (default: `false`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `doctor-finding,doctor-summary,migration-plan,migration-applied`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`.
