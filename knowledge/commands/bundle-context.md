---
type: Command
group: query
mutates: false
title: okf bundle context
description: Inspects effective settings, global guidance, scoped rules, and selected workflow instructions.
---
# okf bundle context

Inspects effective settings, global guidance, scoped rules, and selected workflow instructions.

See the [bundle customization guide](../guides/customize-a-bundle.md) and
[remote dependency guide](../guides/remote-dependencies.md) for selection, guidance, and access boundaries.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | — |
| `--operation <value>` | list<string> | no | — |
| `--workflow <value>` | list<string> | no | — |
| `--all` | bool | no |  (default: `false`) |
| `--inputs <value>` | string | no | JSON input object or @file; values remain typed data |
| `--query <value>` | string | no | JSON query object or @file; resolve without executing the query |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `context-scope,bundle-context,guidance,workflow,effective-query`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

Load global task context before source access, then full applicable command guidance before acting. Context includes effective settings and origins, instruction content/completeness, and an effective digest including loaded instruction files. --all and historical context are inspection, not workflow activation. Missing required instructions block dependent agent work. Incidental guidance output does not replace pre-action loading.
