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
| `--rule <value>` | list<string> | no | Expand a guidance rule for inspection, including optional reference material (repeatable) |
| `--workflow <value>` | list<string> | no | — |
| `--all` | bool | no |  (default: `false`) |
| `--settings-detail <value>` | string | no | Settings projection; full preserves the complete JSON contract, compact shows relevant values (default: `full`) |
| `--stats` | bool | no | Show delivered instruction bytes per section and totals (not estimated tokens) (default: `false`) |
| `--guidance-budget <value>` | string | no | Advisory byte budget for delivered mandatory instructions; never truncates instructions |
| `--token` | bool | no | Issue a stateless context token after complete applicable instruction delivery (default: `false`) |
| `--check-token <value>` | string | no | Check a token without executing an operation; optional --operation checks coverage too |
| `--inputs <value>` | string | no | JSON input object or @file; values remain typed data |
| `--query <value>` | string | no | JSON query object or @file; resolve without executing the query |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `context-scope,bundle-context,guidance,workflow,effective-query,context-stats,context-token,context-check`.

Supported global options: `--bundle-id`, `--bundle-revision`, `--catalog`, `--catalog-scope`, `--guidance`, `--json`, `--revision`, `--scope-bundle`, `--workspace-revision`.

Batch known operations in the initial request to load global and mandatory scoped instructions together. Reference-tier rules expand with --rule; --all and historical context are inspection only. --settings-detail compact explicitly projects settings; full JSON remains the default. --stats reports delivered instruction bytes. --token emits a stateless receipt after delivery; pass it with --context (or @file) on covered commands, or check with --check-token. A stale supplied token fails before execution; tokens are optional and are not authorization or concept-content guards. Context digest version 2 covers loaded instructions and selected workflow resources as well as settings, ontology and snapshot identity. Incidental guidance never replaces pre-action loading.
