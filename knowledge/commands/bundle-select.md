---
type: Command
group: query
mutates: false
title: okf bundle select
description: Resolve recovery-safe current bundle selection and issue a freshness token.
---
# okf bundle select

Resolve current bundle selection without reading bundle contents or loading guidance.
See [selection and scope](../concepts/selection-and-scope.md#retain-a-selection-across-commands)
and [write recovery](../reference/troubleshooting.md#recover-an-interrupted-write).

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `bundle-selection`.

Supported global options: `--bundle-id`, `--catalog`, `--json`.

Resolve an existing bundle root and its identity/access policy without reading transactional bundle
contents. Selection remains available with a pending recovery journal. Retain the returned token and
pass it with --selection (or @file) to subsequent operations, including changeset recover, after
acquiring any caller-owned root lock. The receipt binds the original selection context across
working directories and rejects changed selection inputs. It is a freshness check, not an
authorization credential, content baseline, or global filesystem lock. Load normal bundle context
after recovery and retain --expect for guarded changeset publication.
