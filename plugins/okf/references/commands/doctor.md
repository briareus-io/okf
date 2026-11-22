# okf doctor

Diagnose compatibility and safely repair an existing bundle for OKF v0.2

```sh
okf doctor knowledge
```

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

Output stream: `doctor-finding,doctor-summary,doctor-normalization,migration-plan,migration-applied`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [doctor-normalization](../records/doctor-normalization.md), [migration-plan](../records/migration-plan.md), [migration-applied](../records/migration-applied.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--catalog-scope`, `--context`, `--guidance`, `--json`, `--scope-bundle`, `--selection`.

Inspect migration findings without changing files by default. --selection is incompatible with --migrate-config and --migration-apply because configuration migration has its own targets and guards. Catalog-aware source checks require explicit `--scope-bundle` or `--catalog-scope` for other registered bundles. File, line-range, and markdown-heading source paths use the bundle root, consistently with fingerprinting and graph resolution; kind-less provenance and body links use document-relative paths. A leading slash in a typed file source retains filesystem-absolute semantics, unlike a leading slash in a Markdown link. Correct typed sources that previously relied on graph resolution treating them as document-relative.

`--normalize-relative-links` explicitly chooses current document-relative semantics for bare body links and previews exact mappings and diffs; add `--yes` to publish, while `--dry-run` always previews. It does not infer original author intent and is separate from `--fix-safe`. Only checked existing local targets are normalized; missing, blocked, and unrecognized or ambiguous destination syntax remain for review. Destination-only rewrites preserve frontmatter, source entries, labels, titles, and query/fragment suffixes. Normalization is published before any requested safe repairs, in a separate transaction; the two are not one atomic operation. Review completion evidence if later checks or repairs fail.
