---
type: Command
group: render
mutates: true
title: okf docs
description: 'Generates documentation from a bundle: progressive-disclosure index.md files, or html, md, pdf, graphml, or obsidian output via --format.'
sources:
- resource: crates/okf-cli/src/commands/schema.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
last_modified: 2026-10-04T20:39:29Z
---
# okf docs

Generates documentation from a bundle: progressive-disclosure index.md files, or html, md, pdf, graphml, or obsidian output via --format.
Only `--format index` mutates the bundle. Other formats write an artifact to stdout; with
`--json`, that artifact is wrapped in one `docs` record containing its format and content.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |
| `--format <value>` | string | no | Output format: md|html|graphml|obsidian|index; pdf is retained but unavailable (default: `md`) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `docs,change`.

Supported global options: `--bundle-id`, `--json`.

`--format index` writes indexes throughout the bundle, replacing their bodies; it does not merge curated prose. Use it only when all affected index bodies are generated or replacement is already authorized. Preserve curated indexes and edit only necessary links otherwise. Validate after writes. Other formats emit output rather than updating indexes; the default is `md`.
