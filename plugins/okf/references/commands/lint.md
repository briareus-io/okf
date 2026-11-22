# okf lint

Advisory checks (broken links, missing fields, orphans, ontology violations)

```sh
okf lint knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--baseline <value>` | string | no | Compare findings with a saved lint baseline |
| `--write-baseline <value>` | string | no | Atomically save all findings from a complete lint run |
| `--baseline-ref <value>` | string | no | Compare with the workspace catalog, lock and bundles at this exact local Git revision |
| `--fix` | bool | no | Normalize frontmatter order bundle-wide for the enabled frontmatter-order rule (default: `false`) |
| `--fail-on <value>` | string | no | Severity threshold that makes the run fail (exit 1): never\|info\|warn\|error\|any (default: `error`) |

Output stream: `finding,scope,fix,lint-baseline,lint-comparison,lint-comparison-summary`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [scope](../records/scope.md), [lint-baseline](../records/lint-baseline.md), [lint-comparison](../records/lint-comparison.md), [lint-comparison-summary](../records/lint-comparison-summary.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

For sources with a fingerprint kind, `source-unrecorded` warns when no baseline exists. `source-missing` errors when file, line-range, or markdown-heading sources cannot be fingerprinted, even without a baseline. These source-health rules skip URL fetching and Git fingerprint checks. Configure `source_unrecorded: off` (or `info`, `warn`, `error`; default `warn`) under `settings.lint` in `okf-bundle.yaml`. Legacy TOML bundle settings remain compatibility input. Disabling this rule leaves `source-missing` and `stale` checks active. All lint rule severities accept `off`, `info`, `warn`, or `error`. Defaults: `broken_link` and `source_missing` are `error`; `missing_title`, `spec_v02` (finding rule `okf-v02`), `ontology_violation`, `source_unrecorded`, `invalid_reference`, and `index_coverage` are `warn`; `missing_description` and `orphan` are `info`. Omitted settings keep these defaults. `off` suppresses the selected lint rule only; validation and stale checks remain independent. `index_exclude` controls index coverage exclusions and `finding_budget` limits ontology findings (positive, default 1,000 per concept). The size rules `body_max_lines`, `frontmatter_max_lines`, and `max_line_length` default to `off` and each requires its corresponding positive `_limit` when enabled. CI can select a partial YAML `settings.lint` mapping with `OKF_SETTINGS_OVERRIDES`. Ontology lint reports incompatible top-level `status` declarations once per type, even in empty bundles. Use `--fail-on warn` to fail on warnings as well as errors. With `--baseline` or `--baseline-ref`, that threshold applies to introduced or worsened findings. `--write-baseline` captures all enabled severities.
