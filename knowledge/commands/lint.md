---
type: Command
group: check
mutates: true
title: okf lint
description: 'Advisory checks where the opinions live: broken links, missing descriptions, orphans, and ontology violations, at error/warn/info severities with a --fail-on threshold.'
sources:
- resource: crates/okf-cli/src/commands/check.rs
  kind: git-path
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
- resource: crates/okf-core/src/mutate/gate.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf lint

Advisory checks where the opinions live: broken links, missing descriptions, orphans, and ontology violations, at error/warn/info severities with a --fail-on threshold.

Recursive ontology diagnostics retain the `ontology-violation` rule with concrete field paths
and detailed metadata codes. Invalid schema configuration is separate from advisory findings;
unsupported constraints and depth/finding-budget exhaustion remain visible. Local index-coverage
checks diagnose omitted immediate children and concept-containing directories without rewriting
curated navigation. Catalog-aware checks retain explicit examination scope.

Read-only lint can examine explicit catalog scope. `lint --fix` in CLI 0.5
requires one selected writable bundle and rejects catalog/scope flags. Each
bundle's fixes are a separate transaction; a later failure cannot roll back an
earlier invocation. Display severity does not weaken an independently enabled
ontology write gate.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--fix` | bool | no | Normalize frontmatter order bundle-wide for the enabled frontmatter-order rule (default: `false`) |
| `--fail-on <value>` | string | no | Severity threshold that makes the run fail (exit 1): never|info|warn|error|any (default: `error`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `finding,scope,fix`.

Supported global options: `--bundle-id`, `--catalog-scope`, `--json`, `--scope-bundle`.

For sources with a fingerprint kind, `source-unrecorded` warns when no baseline exists. `source-missing` errors when file, line-range, or markdown-heading sources cannot be fingerprinted, even without a baseline. Lint does not fetch URLs or inspect Git sources. Configure `source_unrecorded = "off"` (or `info`, `warn`, `error`; default `warn`) in `[bundle_settings.default.lint]` in `okf.toml`, or `[bundle_settings."<id>".lint]` for a named bundle. Disabling this rule leaves `source-missing` and `stale` checks active. All lint settings accept `off`, `info`, `warn`, or `error`. Defaults: `broken_link` and `source_missing` are `error`; `missing_title`, `spec_v02` (finding rule `okf-v02`), `ontology_violation`, `source_unrecorded`, and `index_coverage` are `warn`; `missing_description` and `orphan` are `info`. Omitted settings keep these defaults. `off` suppresses the selected lint rule only; validation and stale checks remain independent. `index_exclude` controls index coverage exclusions and `finding_budget` limits ontology findings (positive, default 1,000 per concept). Use `--fail-on warn` to fail on warnings as well as errors.
