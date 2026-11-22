# okf bundle select

Select a bundle without reading transactional bundle contents

```sh
okf bundle select knowledge
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `<bundle>` | positional | no | Bundle directory (explicit, then $OKF_BUNDLE, nearest okf.toml, or current directory) |

Output stream: `bundle-selection`.

NDJSON starts with [operation-start](../records/operation-start.md) and ends with [operation-complete](../records/operation-complete.md). Check the trailer and process exit.

Record shapes: [bundle-selection](../records/bundle-selection.md).

[Global options](../globals.md) supported here: `--bundle-id`, `--catalog`, `--json`.

Resolve an existing bundle root and its identity/access policy without reading transactional bundle contents. Selection remains available with a pending recovery journal. Retain the returned token and pass it with --selection (or @file) to subsequent operations, including changeset recover, after acquiring any caller-owned root lock. The receipt binds the original selection context across working directories and rejects changed selection inputs. It is a freshness check, not an authorization credential, content baseline, or global filesystem lock. Load normal bundle context after recovery and retain --expect for guarded changeset publication.
