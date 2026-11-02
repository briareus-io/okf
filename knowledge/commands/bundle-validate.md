---
type: Command
group: check
mutates: false
title: okf bundle validate
description: Validates manifest structure, supported versions, guidance, and workflow declarations.
---
# okf bundle validate

Validates manifest structure, supported versions, guidance, and workflow declarations.

See the [bundle customization guide](../guides/customize-a-bundle.md) and
[remote dependency guide](../guides/remote-dependencies.md) for selection, guidance, and access boundaries.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `manifest-validation`.

Supported global options: `--bundle-id`, `--catalog`, `--context`, `--guidance`, `--json`.

Validate the optional manifest's structure and supported schema without treating prose instructions as executable checks. Manifest validation is distinct from core concept conformance and bundle-profile knowledge validation.
