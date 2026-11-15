---
type: Command
group: mutate
mutates: true
title: okf changeset recover
description: Recover interrupted publication without overwriting intervening edits.
sources:
- resource: crates/okf-core/src/mutate/transaction.rs
  kind: git-path
- resource: crates/okf-cli/src/cli.rs
  kind: git-path
---
# okf changeset recover

Recover interrupted publication without overwriting intervening edits.

See the [complete CLI reference](index.md) for operation schemas and guarantees.

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Global `--json` requests NDJSON. The optional trailing `bundle` positional is a path; `--bundle-id` selects a catalog identity. Selection uses explicit path or ID, `$OKF_BUNDLE` (path), registered containing bundle, configured default, sole catalog entry, then uncataloged cwd. Selection and examination scope are separate.

Output stream: `changeset-recovery`.

Supported global options: `--bundle-id`, `--catalog`, `--guidance`, `--json`, `--selection`.

Roll back an interrupted uncommitted publication, or finish cleanup of a committed one. Recovery
refuses an active publisher and refuses to overwrite intervening edits. Preserve the journal if
recovery reports a conflict; reconcile those files before retrying. Use bundle select and
--selection to bind the recovery root before acquiring a caller-owned mutex. Recovery skips
guidance; --guidance off is accepted, while full/summary and --context are rejected. Reload normal
context after recovery.
