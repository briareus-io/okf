# okf changeset recover

Recover an interrupted publication without overwriting intervening edits

```sh
okf changeset recover knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `changeset-recovery`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [changeset-recovery](../records/changeset-recovery.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--guidance`, `--json`, `--selection`.

Roll back an interrupted uncommitted publication, or finish cleanup of a committed one. Recovery refuses an active publisher and refuses to overwrite intervening edits. Preserve the journal if recovery reports a conflict; reconcile those files before retrying. Use bundle select and --selection to bind the recovery root before acquiring a caller-owned mutex. Recovery skips guidance; --guidance off is accepted, while full/summary and --context are rejected. Reload normal context after recovery.
