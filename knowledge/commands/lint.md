---
type: Command
group: check
mutates: true
title: okf lint
description: >-
  Advisory checks where the opinions live: broken links, missing descriptions, orphans,
  and ontology violations, at error/warn/info severities with a --fail-on threshold.
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

Advisory checks where the opinions live: broken links, missing descriptions, orphans, and ontology
violations, at error/warn/info severities with a --fail-on threshold.

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

## Ontology lifecycle declarations

Lint reports `ontology-status-conflict` under `ontology-violation` once per incompatible
top-level `status` declaration in each examined bundle, even when the bundle has no records.
The finding gives the ontology path (for example `concepts.Item.fields.status`) and has
`concept: null`. It checks the resolved base type and enum values, including reusable types
and field-level overrides. A text or string field and a compatible enum subset are accepted.
Custom string constraints such as regular expressions are outside this declaration check.

Set severity through `settings.lint.ontology_violation` (default `warn`). CI using
`--fail-on warn` can now fail on a conflicting ontology alone. Concept-level lifecycle and
ontology findings still describe individual records. For field meanings and examples, see
[standard frontmatter fields](../concepts/bundles-and-concepts.md#standard-frontmatter-fields).

## Concept size

Enable any of these rules under `settings.lint` in the bundle manifest. Each defaults to
`off` and requires a positive integer limit when enabled. Setting only a limit preserves the
severity. Each enabled rule emits at most one finding per concept.

| Rule | Severity key | Limit key | Measurement |
|------|--------------|-----------|-------------|
| `body-max-lines` | `body_max_lines` | `body_max_lines_limit` | Physical body lines, including interior blank lines and excluding trailing blank lines |
| `frontmatter-max-lines` | `frontmatter_max_lines` | `frontmatter_max_lines_limit` | Physical YAML lines between the two delimiters, including comments and blank lines |
| `max-line-length` | `max_line_length` | `max_line_length_limit` | Unicode scalar values per line, excluding line endings |

A measurement equal to the limit passes. Without frontmatter, the whole file is the body.
Line-length findings report the total overlong lines and the first three physical line numbers
(one-based from the beginning of the file). Set `max_line_length_scope` to `body` (default),
`frontmatter`, or `all`. In the body, fenced code blocks, Markdown tables, and lines consisting
of one Markdown link or HTTP(S) URL are excluded by default. Set
`max_line_length_exclude_code_blocks`, `max_line_length_exclude_tables`, or
`max_line_length_exclude_links` to `false` to count those lines.

Authors decide whether to split a concept, shorten prose, or adjust the limit. Size findings
have no automatic fix. For CI-specific limits or severities, see the
[configuration reference](../reference/configuration.md#ci-lint-overrides).

## Compare lint findings

Use a saved baseline when a team wants to review and accept existing findings. Capture the full
scope once, then run the comparison on each change:

```sh
okf lint --catalog-scope --write-baseline okf-lint-baseline.json
okf lint --catalog-scope --baseline okf-lint-baseline.json --fail-on warn
```

Capture includes every enabled severity and succeeds when the run is complete. It writes
deterministic versioned JSON and replaces the destination atomically. Comparison leaves that file
untouched. Resolved entries disappear on the next explicit capture. Keep the baseline under version
control if updates need review.

For branch CI, use an exact local revision instead of maintaining a file:

```sh
base=$(git merge-base HEAD origin/main) && okf lint --catalog-scope --baseline-ref "$base" --fail-on warn
```

The current side includes working-tree edits. Each side reads its own tracked workspace
settings, catalog and dependency pins, using the running CLI and the same invocation-level
`OKF_SETTINGS_OVERRIDES`. The comparison reports the resolved base commit. A synthetic
merge build can pass its target parent as the base. The required history and pinned snapshots
must be available locally. Historical comparison of an external mutable directory requires
an identifiable snapshot; a file baseline works for directories outside Git.

Use the same scope for capture and comparison. Registered bundles use catalog IDs.
Catalog-wide comparisons allow membership changes and inspect consumers even when their
files are unchanged. A standalone bundle uses its repository-relative path, or its
directory name outside Git. Baseline modes are mutually exclusive and exclude `--fix`.

The comparison groups occurrences by rule, bundle, concept and rule-specific evidence
such as field, unresolved target or missing index child. Message text and Markdown body
line numbers are display details. An additional occurrence is introduced; a severity
increase is worsened. An existing issue that drops in severity is improved. Only introduced
or worsened findings meeting `--fail-on` cause exit 1 (default `error`). Incomplete input
produces an unsuccessful result even with `--fail-on never`, and preserves a previous
baseline file. JSON includes `lint-comparison` records for all classifications and a
`lint-comparison-summary` with counts and the bundle versions on each side.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--baseline <value>` | string | no | Compare findings with a saved lint baseline |
| `--write-baseline <value>` | string | no | Atomically save all findings from a complete lint run |
| `--baseline-ref <value>` | string | no | Compare with the workspace catalog, lock and bundles at this exact local Git revision |
| `--fix` | bool | no | Normalize frontmatter order bundle-wide for the enabled frontmatter-order rule (default: `false`) |
| `--fail-on <value>` | string | no | Severity threshold that makes the run fail (exit 1): never\|info\|warn\|error\|any (default: `error`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `finding,scope,fix,lint-baseline,lint-comparison,lint-comparison-summary`.

Supported global options: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

For sources with a fingerprint kind, `source-unrecorded` warns when no baseline exists. `source-missing` errors when file, line-range, or markdown-heading sources cannot be fingerprinted, even without a baseline. These source-health rules skip URL fetching and Git fingerprint checks. Configure `source_unrecorded: off` (or `info`, `warn`, `error`; default `warn`) under `settings.lint` in `okf-bundle.yaml`. Legacy TOML bundle settings remain compatibility input. Disabling this rule leaves `source-missing` and `stale` checks active. All lint rule severities accept `off`, `info`, `warn`, or `error`. Defaults: `broken_link` and `source_missing` are `error`; `missing_title`, `spec_v02` (finding rule `okf-v02`), `ontology_violation`, `source_unrecorded`, `invalid_reference`, and `index_coverage` are `warn`; `missing_description` and `orphan` are `info`. Omitted settings keep these defaults. `off` suppresses the selected lint rule only; validation and stale checks remain independent. `index_exclude` controls index coverage exclusions and `finding_budget` limits ontology findings (positive, default 1,000 per concept). The size rules `body_max_lines`, `frontmatter_max_lines`, and `max_line_length` default to `off` and each requires its corresponding positive `_limit` when enabled. CI can select a partial YAML `settings.lint` mapping with `OKF_SETTINGS_OVERRIDES`. Ontology lint reports incompatible top-level `status` declarations once per type, even in empty bundles. Use `--fail-on warn` to fail on warnings as well as errors. With `--baseline` or `--baseline-ref`, that threshold applies to introduced or worsened findings. `--write-baseline` captures all enabled severities.
